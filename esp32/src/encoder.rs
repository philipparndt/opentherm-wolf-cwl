//! Rotary encoder input — interrupt-driven Gray-code transition counter.
//!
//! Each ISR computes the direction of one Gray-code edge from the previous
//! and current pin states (`+1` = CW, `-1` = CCW, `0` = no/illegal change)
//! via a `(prev, new)`-indexed lookup table. A signed counter accumulates;
//! reaching `±4` emits one clean step. Partial clicks (the encoder physically
//! advanced past the mid-point but the spring snapped back without closing
//! the cycle) are emitted by `poll()` after a ~100 ms quiescence window,
//! based on the peak counter value reached during the gesture. A 50 ms
//! reverse-suppress window swallows opposite-direction emits caused by
//! contact bounce-back right after a clean click.
//!
//! Compared to the previous Buxton state machine, this decoder does *not*
//! assume the encoder rests at any particular pinstate. That matters on
//! cheap mechanical encoders whose rest state varies across detents due
//! to rotor/contact misalignment — we observed a unit where one detent
//! routinely left the encoder physically stuck at pinstate `10` after a
//! click, which caused the state machine to mis-attribute the *next*
//! gesture's direction.
//!
//! NOTE: we deliberately bypass `PinDriver::subscribe`/`enable_interrupt`. The
//! esp-idf-hal ISR trampoline calls `gpio_intr_disable` after every fire (to
//! guard against IWDT triggers on level-triggered pins), which requires
//! re-arming from non-ISR context — fatal for an encoder, since edges between
//! disable and re-arm are lost. We register our own handler via the raw
//! ESP-IDF C API; edge-triggered interrupts then keep re-firing automatically.

use esp_idf_svc::hal::gpio::{
    enable_isr_service, AnyIOPin, Input, InterruptType, Pin, PinDriver, Pull,
};
use esp_idf_svc::sys::{
    esp, esp_timer_get_time, gpio_get_level, gpio_intr_enable, gpio_isr_handler_add,
};
use log::info;
use std::sync::atomic::{AtomicI8, AtomicI32, AtomicU8, AtomicU32, AtomicUsize, Ordering};

/// Flip to `true` to dump every raw CLK/DT transition from the ISR ring.
/// Useful when diagnosing a bouncy encoder but very noisy in normal use.
/// Per-emit `ENC rot EMIT` / `ENC press EMIT` lines are logged unconditionally.
const DEBUG: bool = false;
const DEBUG_RING_LEN: usize = 64;

/// No edges for this long → poll() considers a partial emit. Must be longer
/// than realistic intra-click intervals (a slow but normal click puts ~100–
/// 300 ms between adjacent Gray-code edges); too short and a slow click
/// trips the partial path twice — once mid-stroke and once at the end.
const QUIESCENT_MS: u32 = 500;

/// Peak |count| during a gesture must reach this to qualify for a partial
/// emit. 4 = a full click; 3 means the encoder physically advanced through
/// three of the four Gray-code edges (i.e. one edge short of completing).
/// We deliberately don't honour `2` ("just past the midpoint") because that
/// rounds slow clicks up by one too often.
const HALFWAY_THRESHOLD: i32 = 3;

/// Opposite-direction partial emits within this window of the previous
/// emit are dropped as contact bounce-back rather than a real reversal.
/// Long enough to cover both the bounce itself (~50–200 ms) and the
/// subsequent QUIESCENT_MS wait before poll() makes the partial-emit
/// decision (otherwise the suppress check expires too soon).
const REVERSE_SUPPRESS_MS: u32 = 500;


/// Direction of a single pinstate transition, indexed by `[prev][new]`
/// (each 0..=3 = `(CLK<<1) | DT`).  `+1` = one CW Gray-code edge,
/// `-1` = one CCW edge, `0` = unchanged or an "illegal" diagonal jump
/// (both bits flipped in one sample — happens occasionally with very
/// fast input or coalesced interrupts).
///
/// CW Gray-code cycle:  `11 → 01 → 00 → 10 → 11`
/// CCW Gray-code cycle: `11 → 10 → 00 → 01 → 11`
#[rustfmt::skip]
const TRANS_DIR: [[i8; 4]; 4] = [
    //                  new=00    new=01    new=10    new=11
    /* prev=00 */    [     0,       -1,        1,        0  ],
    /* prev=01 */    [     1,        0,        0,       -1  ],
    /* prev=10 */    [    -1,        0,        0,        1  ],
    /* prev=11 */    [     0,        1,       -1,        0  ],
];

#[derive(Debug, Clone, Copy)]
pub enum EncoderEvent {
    Rotate(i32),   // +1 = clockwise, -1 = counter-clockwise
    Press,
}

/// Shared between the GPIO ISR and the main thread. All ISR-mutated fields
/// are atomic; the gpio numbers are read-only after construction.
struct IsrState {
    // Pin levels at the previous ISR fire. Swapped atomically with the
    // current sample on each fire to compute the direction of one edge.
    last_pinstate: AtomicU8,

    // Signed net transition count since the last gesture reset.  `±4` is
    // one complete physical click and triggers a clean emit.
    count: AtomicI32,

    // Furthest CW (positive) and CCW (negative) values `count` reached
    // since the last reset.  Used by the quiescence-based partial emit
    // so that a click that *almost* completed (e.g. peak `+3` then back
    // to `0`) still registers as one step.
    max_cw_count: AtomicI32,
    min_ccw_count: AtomicI32,

    // Time of the most recent direction-bearing transition. `0` means
    // "no gesture in flight"; poll() uses (now - this) > QUIESCENT_MS
    // to decide a gesture is over.
    last_transition_ms: AtomicU32,

    // Queue of decoded steps drained by `poll()`.
    steps: AtomicI32,

    // Timestamp + direction of the most recent emit (clean or partial).
    // Drives the reverse-suppress window. `last_emit_dir`: 0 = none,
    // 1 = CW, -1 = CCW.
    last_emit_ms: AtomicU32,
    last_emit_dir: AtomicI8,

    clk_gpio: i32,
    dt_gpio: i32,

    // Diagnostic ring. The ISR is the sole writer (GPIO ISRs don't preempt
    // themselves); the main thread is the sole reader. Wrap-overruns are
    // detected via `head - tail > LEN` and reported as dropped entries.
    debug_buf: [AtomicU32; DEBUG_RING_LEN],
    debug_head: AtomicUsize,
    debug_tail: AtomicUsize,
}

pub struct Encoder {
    _clk: PinDriver<'static, AnyIOPin, Input>,
    _dt: PinDriver<'static, AnyIOPin, Input>,
    sw: PinDriver<'static, AnyIOPin, Input>,

    // Leaked so the ISR can hold a stable pointer for the program's lifetime.
    // Encoder is constructed once in main() and never dropped, so this is fine.
    isr: &'static IsrState,

    last_button: bool,
    button_down_since: Option<u32>,
    last_button_event_ms: u32,
    button_debounce_ms: u32,
}

impl Encoder {
    pub fn new(
        clk_pin: AnyIOPin,
        dt_pin: AnyIOPin,
        sw_pin: AnyIOPin,
    ) -> Result<Self, esp_idf_svc::sys::EspError> {
        let clk_gpio = clk_pin.pin();
        let dt_gpio = dt_pin.pin();

        let mut clk = PinDriver::input(clk_pin)?;
        clk.set_pull(Pull::Up)?;
        clk.set_interrupt_type(InterruptType::AnyEdge)?;

        let mut dt = PinDriver::input(dt_pin)?;
        dt.set_pull(Pull::Up)?;
        dt.set_interrupt_type(InterruptType::AnyEdge)?;

        let mut sw = PinDriver::input(sw_pin)?;
        sw.set_pull(Pull::Up)?;

        // Sample initial pin levels so the first ISR's "previous pinstate"
        // matches the actual rest state of the encoder. Without this we'd
        // mis-compute the very first edge's direction.
        let initial_pinstate =
            ((clk.is_high() as u8) << 1) | (dt.is_high() as u8);

        let isr: &'static IsrState = Box::leak(Box::new(IsrState {
            last_pinstate: AtomicU8::new(initial_pinstate),
            count: AtomicI32::new(0),
            max_cw_count: AtomicI32::new(0),
            min_ccw_count: AtomicI32::new(0),
            last_transition_ms: AtomicU32::new(0),
            steps: AtomicI32::new(0),
            last_emit_ms: AtomicU32::new(0),
            last_emit_dir: AtomicI8::new(0),
            clk_gpio,
            dt_gpio,
            debug_buf: core::array::from_fn(|_| AtomicU32::new(0)),
            debug_head: AtomicUsize::new(0),
            debug_tail: AtomicUsize::new(0),
        }));

        // Bypass esp-idf-hal's auto-disable trampoline — register our own
        // handler so edge-triggered interrupts keep re-firing.
        enable_isr_service()?;
        let isr_ptr = isr as *const IsrState as *mut core::ffi::c_void;
        unsafe {
            esp!(gpio_isr_handler_add(clk_gpio, Some(quad_isr_trampoline), isr_ptr))?;
            esp!(gpio_isr_handler_add(dt_gpio, Some(quad_isr_trampoline), isr_ptr))?;
            esp!(gpio_intr_enable(clk_gpio))?;
            esp!(gpio_intr_enable(dt_gpio))?;
        }

        let last_button = sw.is_high();
        info!(
            "Encoder: Initialized with interrupts (CLK=GPIO{} DT=GPIO{} SW={})",
            clk_gpio, dt_gpio, last_button as u8
        );

        Ok(Self {
            _clk: clk,
            _dt: dt,
            sw,
            isr,
            last_button,
            button_down_since: None,
            last_button_event_ms: 0,
            button_debounce_ms: 30,
        })
    }

    /// Poll for one pending event. Returns `None` when nothing is queued.
    /// The main loop should drain this with `while let Some(_) = enc.poll()`,
    /// so a fast burst of rotation gets processed in one iteration.
    pub fn poll(&mut self, now_ms: u32) -> Option<EncoderEvent> {
        if DEBUG {
            self.drain_debug();
        }

        // Re-read the clock fresh. `drain_encoder` calls us in a while-let
        // loop with the same `now_ms` snapshot taken at the top of the main
        // loop iteration; meanwhile each `disp.update()` between calls eats
        // ~25 ms of wall time and the ISR keeps firing. If we used the stale
        // `now_ms` against the ISR-set `last_transition_ms` (which uses real
        // time), the subtraction underflowed and the quiescent check fired
        // prematurely on every poll, double-emitting partial steps. The
        // button/log paths below still use the passed snapshot for log
        // consistency across one drain pass — they don't compare against
        // ISR-supplied timestamps.
        let real_now_ms = (unsafe { esp_timer_get_time() } / 1000) as u32;

        // Quiescence-based partial emit: if the encoder has had no edge for
        // QUIESCENT_MS, decide whether the gesture's peak progress was big
        // enough to count as one (partial) click and queue a step. Then
        // reset all gesture-scoped accumulators so the next click starts
        // fresh.
        let last_trans = self.isr.last_transition_ms.load(Ordering::Relaxed);
        if last_trans != 0 && real_now_ms.wrapping_sub(last_trans) >= QUIESCENT_MS {
            let cw_p = self.isr.max_cw_count.load(Ordering::Relaxed).max(0);
            let ccw_p = (-self.isr.min_ccw_count.load(Ordering::Relaxed)).max(0);
            let candidate: i8 =
                if cw_p >= HALFWAY_THRESHOLD && cw_p > ccw_p { 1 }
                else if ccw_p >= HALFWAY_THRESHOLD && ccw_p > cw_p { -1 }
                else { 0 };
            if candidate != 0 {
                let last_emit_ms = self.isr.last_emit_ms.load(Ordering::Relaxed);
                let last_emit_dir = self.isr.last_emit_dir.load(Ordering::Relaxed);
                let opposite =
                    (candidate == 1 && last_emit_dir == -1) ||
                    (candidate == -1 && last_emit_dir == 1);
                let elapsed = real_now_ms.wrapping_sub(last_emit_ms);
                if !(opposite && elapsed < REVERSE_SUPPRESS_MS) {
                    self.isr.steps.fetch_add(candidate as i32, Ordering::Relaxed);
                    self.isr.last_emit_ms.store(real_now_ms, Ordering::Relaxed);
                    self.isr.last_emit_dir.store(candidate, Ordering::Relaxed);
                    if DEBUG {
                        info!(
                            "ENC partial EMIT t={} dir={} cw_peak={} ccw_peak={}",
                            real_now_ms, candidate, cw_p, ccw_p
                        );
                    }
                } else if DEBUG {
                    info!(
                        "ENC partial SUPPRESSED (reverse) t={} dir={} elapsed={}ms",
                        real_now_ms, candidate, elapsed
                    );
                }
            }
            // Reset gesture state whether we emitted or not.
            self.isr.count.store(0, Ordering::Relaxed);
            self.isr.max_cw_count.store(0, Ordering::Relaxed);
            self.isr.min_ccw_count.store(0, Ordering::Relaxed);
            self.isr.last_transition_ms.store(0, Ordering::Relaxed);
        }

        // One step per call: emit ±1 and decrement the accumulator. The sign
        // tells the consumer the direction; magnitude > 1 is impossible.
        let steps = self.isr.steps.load(Ordering::Relaxed);
        if steps != 0 {
            let delta = if steps > 0 { 1 } else { -1 };
            self.isr.steps.fetch_sub(delta, Ordering::Relaxed);
            if DEBUG {
                info!("ENC rot EMIT t={} delta={} (pending={})", now_ms, delta, steps);
            }
            return Some(EncoderEvent::Rotate(delta));
        }

        // Button — still polled. Debounced against repeated samples within
        // `button_debounce_ms`.
        let button = self.sw.is_high();
        if !button && self.last_button {
            self.button_down_since = Some(now_ms);
            if DEBUG { info!("ENC btn DOWN t={}", now_ms); }
        } else if button && !self.last_button {
            if let Some(down_since) = self.button_down_since {
                self.button_down_since = None;
                let held = now_ms.wrapping_sub(down_since);
                if held < 10_000 {
                    if now_ms.wrapping_sub(self.last_button_event_ms) >= self.button_debounce_ms {
                        self.last_button_event_ms = now_ms;
                        self.last_button = button;
                        if DEBUG { info!("ENC press EMIT t={} held={}ms", now_ms, held); }
                        return Some(EncoderEvent::Press);
                    } else if DEBUG {
                        info!("ENC press SUPPRESSED (debounce) t={} held={}ms", now_ms, held);
                    }
                } else if DEBUG {
                    info!("ENC btn UP t={} held={}ms (long, handled elsewhere)", now_ms, held);
                }
            }
        }
        self.last_button = button;

        None
    }

    fn drain_debug(&self) {
        let head = self.isr.debug_head.load(Ordering::Acquire);
        let mut tail = self.isr.debug_tail.load(Ordering::Relaxed);

        let queued = head.wrapping_sub(tail);
        if queued > DEBUG_RING_LEN {
            let dropped = queued - DEBUG_RING_LEN;
            info!("ENC isr: dropped {} entries (ring overflow)", dropped);
            tail = head.wrapping_sub(DEBUG_RING_LEN);
        }

        while tail != head {
            let entry = self.isr.debug_buf[tail % DEBUG_RING_LEN].load(Ordering::Relaxed);
            let t = entry & 0x00FF_FFFF;
            let clk = (entry >> 24) & 1;
            let dt = (entry >> 25) & 1;
            let trans = (entry >> 26) & 0x3;
            let trans_s = match trans { 1 => "+1", 2 => "-1", _ => " 0" };
            // Sign-extend the 4-bit count.
            let raw = ((entry >> 28) & 0xF) as i32;
            let count = if raw & 0x8 != 0 { raw - 16 } else { raw };
            info!(
                "ENC isr t={} CLK={} DT={} trans={} count={}",
                t, clk, dt, trans_s, count
            );
            tail = tail.wrapping_add(1);
        }
        self.isr.debug_tail.store(tail, Ordering::Release);
    }

    /// Check if the button is being held for 10+ seconds (factory reset).
    pub fn is_long_hold(&self, now_ms: u32) -> bool {
        if let Some(down_since) = self.button_down_since {
            now_ms.wrapping_sub(down_since) >= 10_000
        } else {
            false
        }
    }
}

// C entry point registered directly with the ESP-IDF GPIO ISR service. Critically,
// this trampoline does NOT call gpio_intr_disable, so any-edge interrupts keep
// firing on subsequent edges without needing to be re-armed from a task.
unsafe extern "C" fn quad_isr_trampoline(arg: *mut core::ffi::c_void) {
    let state = &*(arg as *const IsrState);
    on_quad_isr(state);
}

// Runs in interrupt context. Must stay short and allocation-free; no logging
// (printf locks). Pin reads use gpio_get_level (IRAM-safe register read).
fn on_quad_isr(isr: &IsrState) {
    let clk = unsafe { gpio_get_level(isr.clk_gpio as _) };
    let dt = unsafe { gpio_get_level(isr.dt_gpio as _) };
    let new_ps = (((clk & 1) << 1) | (dt & 1)) as u8;

    let prev_ps = isr.last_pinstate.swap(new_ps, Ordering::Relaxed);
    let dir_step = TRANS_DIR[prev_ps as usize][new_ps as usize] as i32;

    let now_ms = (unsafe { esp_timer_get_time() } / 1000) as u32;

    let mut new_count = isr.count.load(Ordering::Relaxed);
    let mut emit_dir: i8 = 0;

    if dir_step != 0 {
        isr.last_transition_ms.store(now_ms, Ordering::Relaxed);
        new_count = isr.count.fetch_add(dir_step, Ordering::Relaxed) + dir_step;

        // Track the peak excursion for the partial-emit decision in poll().
        if new_count > 0 {
            isr.max_cw_count.fetch_max(new_count, Ordering::Relaxed);
        } else if new_count < 0 {
            isr.min_ccw_count.fetch_min(new_count, Ordering::Relaxed);
        }

        // Clean ±4 emit. Reset count + peaks so the next click starts from
        // zero. Record the emit for the reverse-suppress window.
        if new_count >= 4 {
            isr.count.fetch_sub(4, Ordering::Relaxed);
            isr.max_cw_count.store(0, Ordering::Relaxed);
            isr.min_ccw_count.store(0, Ordering::Relaxed);
            isr.steps.fetch_add(1, Ordering::Relaxed);
            isr.last_emit_ms.store(now_ms, Ordering::Relaxed);
            isr.last_emit_dir.store(1, Ordering::Relaxed);
            emit_dir = 1;
        } else if new_count <= -4 {
            isr.count.fetch_add(4, Ordering::Relaxed);
            isr.max_cw_count.store(0, Ordering::Relaxed);
            isr.min_ccw_count.store(0, Ordering::Relaxed);
            isr.steps.fetch_sub(1, Ordering::Relaxed);
            isr.last_emit_ms.store(now_ms, Ordering::Relaxed);
            isr.last_emit_dir.store(-1, Ordering::Relaxed);
            emit_dir = -1;
        }
    }

    if DEBUG {
        // Layout (one u32 per ring entry):
        //   bits  0..23 — t_ms (24-bit wrap, ~4.6 hours)
        //   bit      24 — CLK
        //   bit      25 — DT
        //   bits 26..27 — trans dir   (0 none, 1 CW edge, 2 CCW edge)
        //   bits 28..31 — count, signed 4-bit two's-complement, clamped
        //                 to [-8, 7] (sufficient — count is reset on
        //                 every ±4 emit so it rarely exceeds 3)
        let t_ms = unsafe { esp_timer_get_time() / 1000 } as u32;
        let trans_bits: u32 = match dir_step { 1 => 1, -1 => 2, _ => 0 };
        let count_bits: u32 = (new_count.clamp(-8, 7) as u32) & 0xF;
        let _ = emit_dir; // implicit: emit fires only on a transition, and we
                          // can see it from the next entry's count snapping
                          // back toward zero.
        let entry = (t_ms & 0x00FF_FFFF)
            | (((clk as u32) & 1) << 24)
            | (((dt as u32) & 1) << 25)
            | (trans_bits << 26)
            | (count_bits << 28);
        let pos = isr.debug_head.load(Ordering::Relaxed);
        isr.debug_buf[pos % DEBUG_RING_LEN].store(entry, Ordering::Relaxed);
        isr.debug_head.store(pos.wrapping_add(1), Ordering::Release);
    }
}
