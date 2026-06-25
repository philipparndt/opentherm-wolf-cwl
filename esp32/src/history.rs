//! 24 h temperature history — three RAM-only ring buffers of 128 buckets,
//! each holding min/max over a ~11.25 min window. Sampled once per second
//! from the main loop.

use crate::cwl_data::CwlData;

pub const HISTORY_SLOTS: usize = 128;
pub const BUCKET_MS: u32 = (24 * 60 * 60 * 1000 / HISTORY_SLOTS) as u32; // 675_000 ms
pub const SAMPLE_INTERVAL_MS: u32 = 1_000;

#[derive(Debug, Clone, Copy)]
pub struct Bucket {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug)]
pub struct Channel {
    pub slots: [Option<Bucket>; HISTORY_SLOTS],
    pub head: usize,
}

impl Channel {
    pub fn new() -> Self {
        Self { slots: [None; HISTORY_SLOTS], head: 0 }
    }

    /// Fold a sample into the current head slot. Initialises the slot on the
    /// first sample of a new bucket, updates min/max otherwise.
    pub fn fold(&mut self, value: f32) {
        match self.slots[self.head] {
            None => self.slots[self.head] = Some(Bucket { min: value, max: value }),
            Some(ref mut b) => {
                if value < b.min { b.min = value; }
                if value > b.max { b.max = value; }
            }
        }
    }

    /// Advance head to the next slot, clearing it so the rolling window
    /// discards the bucket that's about to be reused.
    pub fn advance(&mut self) {
        self.head = (self.head + 1) % HISTORY_SLOTS;
        self.slots[self.head] = None;
    }

    /// Aggregate min/max across all populated slots. Returns `None` if no
    /// slot has data yet (cold start).
    pub fn min_max(&self) -> Option<(f32, f32)> {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        let mut any = false;
        for slot in &self.slots {
            if let Some(b) = slot {
                if b.min < lo { lo = b.min; }
                if b.max > hi { hi = b.max; }
                any = true;
            }
        }
        if any { Some((lo, hi)) } else { None }
    }

    /// Read slots in display order: column 0 = oldest, column 127 = newest.
    /// Newest is the slot we're currently aggregating into (head).
    pub fn slot_at_column(&self, col: usize) -> Option<Bucket> {
        // head is the slot we're currently writing → it's the newest.
        // column 127 = head, column 126 = head-1, ..., column 0 = head+1 (wrapping).
        let idx = (self.head + HISTORY_SLOTS - (HISTORY_SLOTS - 1 - col)) % HISTORY_SLOTS;
        self.slots[idx]
    }
}

/// Synthetic-summer-day temperature snapshot used by both the dev-mode
/// history preload and the OpenTherm simulator so the chart shows one
/// continuous curve across the preload→live boundary.
#[cfg(feature = "simulate-ot")]
pub struct SimDayCycle {
    pub outdoor: f32,
    pub indoor: f32,
    pub supply_outlet: f32,
    pub exhaust_outlet: f32,
    pub delta: f32,
}

#[cfg(feature = "simulate-ot")]
pub const SIM_PEAK_HOUR: f32 = 14.0;

/// Evaluate the simulated summer-day cycle. `hours_offset` is hours since
/// boot (use negative values for the preload to project backwards). At
/// offset 0 the firmware boots at `SIM_PEAK_HOUR` (14:00 — outdoor peak).
#[cfg(feature = "simulate-ot")]
pub fn simulated_day_cycle(hours_offset: f32) -> SimDayCycle {
    use core::f32::consts::PI;
    let outdoor_base = 20.0_f32;
    let outdoor_amp = 8.0_f32;
    let indoor_base = 22.0_f32;
    let indoor_amp = 0.5_f32;
    let efficiency = 0.85_f32;

    let hour_of_day = (SIM_PEAK_HOUR + hours_offset).rem_euclid(24.0);
    let phase = (hour_of_day - SIM_PEAK_HOUR) * (2.0 * PI / 24.0);
    let outdoor = outdoor_base + outdoor_amp * phase.cos();
    let indoor = indoor_base + indoor_amp * (phase + PI / 4.0).sin();
    let supply_outlet = outdoor + (indoor - outdoor) * efficiency;
    let exhaust_outlet = indoor - (indoor - outdoor) * efficiency;
    let delta = supply_outlet - outdoor;
    SimDayCycle { outdoor, indoor, supply_outlet, exhaust_outlet, delta }
}

#[derive(Debug)]
pub struct TempHistory {
    pub outdoor: Channel,
    pub indoor: Channel,
    pub delta: Channel,
    bucket_start_ms: u32,
    last_sample_ms: u32,
}

impl TempHistory {
    pub fn new() -> Self {
        let h = Self {
            outdoor: Channel::new(),
            indoor: Channel::new(),
            delta: Channel::new(),
            bucket_start_ms: 0,
            last_sample_ms: 0,
        };
        #[cfg(feature = "simulate-ot")]
        return Self::preloaded_simulation(h);
        #[cfg(not(feature = "simulate-ot"))]
        h
    }

    /// Backfill 24 h of synthetic data so the dev-mode chart pages have
    /// something interesting on boot. Column 127 ("now") is anchored at
    /// `SIM_PEAK_HOUR`; `simulated_day_cycle` produces the exact same curve
    /// that `ot_master::simulate` uses, so the preload and the live
    /// samples form a continuous trace.
    #[cfg(feature = "simulate-ot")]
    fn preloaded_simulation(mut h: Self) -> Self {
        let slot_hours: f32 = 24.0 / HISTORY_SLOTS as f32;
        for col in 0..HISTORY_SLOTS {
            let hours_ago = (HISTORY_SLOTS - 1 - col) as f32 * slot_hours;
            let cycle = simulated_day_cycle(-hours_ago);
            // Column-to-slot mapping mirrors Channel::slot_at_column with head=0.
            let idx = (HISTORY_SLOTS + col + 1) % HISTORY_SLOTS;
            h.outdoor.slots[idx] = Some(Bucket { min: cycle.outdoor, max: cycle.outdoor });
            h.indoor.slots[idx] = Some(Bucket { min: cycle.indoor, max: cycle.indoor });
            h.delta.slots[idx] = Some(Bucket { min: cycle.delta, max: cycle.delta });
        }
        h
    }

    /// Fold the latest temperatures into the active bucket. Throttled to
    /// `SAMPLE_INTERVAL_MS`. Returns `true` when a bucket boundary was
    /// crossed (so the display can be marked dirty).
    pub fn sample(&mut self, now_ms: u32, data: &CwlData) -> bool {
        if self.last_sample_ms != 0
            && now_ms.wrapping_sub(self.last_sample_ms) < SAMPLE_INTERVAL_MS
        {
            return false;
        }
        self.last_sample_ms = now_ms;

        let mut rolled = false;
        if self.bucket_start_ms == 0 {
            self.bucket_start_ms = now_ms;
        } else if now_ms.wrapping_sub(self.bucket_start_ms) >= BUCKET_MS {
            self.outdoor.advance();
            self.indoor.advance();
            self.delta.advance();
            self.bucket_start_ms = now_ms;
            rolled = true;
        }

        self.outdoor.fold(data.supply_inlet_temp);
        self.indoor.fold(data.exhaust_inlet_temp);
        if data.supports_id81 {
            self.delta.fold(data.supply_outlet_temp - data.supply_inlet_temp);
        }

        rolled
    }

    /// Serialize the three channels (oldest→newest) to a compact JSON snapshot
    /// for retained MQTT persistence. Each bucket is `[min,max]` or `null`.
    /// Shape matches the `/api/history` endpoint so the frontend format is reused.
    pub fn snapshot_json(&self, now_epoch: i64) -> String {
        format!(
            "{{\"bucketMs\":{},\"nowEpoch\":{},\"supply\":{},\"exhaust\":{},\"delta\":{}}}",
            BUCKET_MS,
            now_epoch,
            channel_to_json(&self.outdoor),
            channel_to_json(&self.indoor),
            channel_to_json(&self.delta),
        )
    }

    /// Restore the channels from a snapshot produced by [`snapshot_json`].
    /// Buckets are placed oldest→newest with the newest as the current head, so
    /// live sampling continues into it. Malformed/empty input is a no-op (the
    /// current in-RAM data is kept). Returns `true` when something was restored.
    pub fn restore_from_snapshot(&mut self, json: &[u8]) -> bool {
        let val: serde_json::Value = match serde_json::from_slice(json) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let mut any = false;
        any |= restore_channel(&mut self.outdoor, &val["supply"]);
        any |= restore_channel(&mut self.indoor, &val["exhaust"]);
        any |= restore_channel(&mut self.delta, &val["delta"]);
        if any {
            // Continue sampling into the restored newest bucket rather than
            // immediately rolling over.
            self.bucket_start_ms = 0;
            self.last_sample_ms = 0;
        }
        any
    }
}

/// Serialize one channel as a JSON array of `[min,max]` / `null`, oldest→newest.
fn channel_to_json(ch: &Channel) -> String {
    let mut s = String::with_capacity(HISTORY_SLOTS * 12);
    s.push('[');
    for col in 0..HISTORY_SLOTS {
        if col > 0 { s.push(','); }
        match ch.slot_at_column(col) {
            Some(b) => s.push_str(&format!("[{:.2},{:.2}]", b.min, b.max)),
            None => s.push_str("null"),
        }
    }
    s.push(']');
    s
}

/// Rebuild a channel from a JSON array of `[min,max]` / `null` (oldest→newest).
/// Sets `head` to the newest slot so `slot_at_column` reads back identically.
fn restore_channel(ch: &mut Channel, arr: &serde_json::Value) -> bool {
    let items = match arr.as_array() {
        Some(a) => a,
        None => return false,
    };
    // With head at the last slot, slot_at_column(col) maps to slots[col].
    ch.head = HISTORY_SLOTS - 1;
    ch.slots = [None; HISTORY_SLOTS];
    let n = items.len().min(HISTORY_SLOTS);
    for col in 0..n {
        if let Some(pair) = items[col].as_array() {
            if pair.len() == 2 {
                if let (Some(min), Some(max)) = (pair[0].as_f64(), pair[1].as_f64()) {
                    ch.slots[col] = Some(Bucket { min: min as f32, max: max as f32 });
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_two_samples_updates_min_max() {
        let mut ch = Channel::new();
        ch.fold(20.0);
        ch.fold(15.0);
        ch.fold(25.0);
        let b = ch.slots[ch.head].unwrap();
        assert!((b.min - 15.0).abs() < 1e-6);
        assert!((b.max - 25.0).abs() < 1e-6);
    }

    #[test]
    fn advance_then_fold_starts_fresh_bucket() {
        let mut ch = Channel::new();
        ch.fold(20.0);
        ch.advance();
        ch.fold(10.0);
        let b = ch.slots[ch.head].unwrap();
        assert!((b.min - 10.0).abs() < 1e-6);
        assert!((b.max - 10.0).abs() < 1e-6);
    }

    #[test]
    fn min_max_over_all_none_returns_none() {
        let ch = Channel::new();
        assert!(ch.min_max().is_none());
    }

    #[test]
    fn min_max_aggregates_populated_slots() {
        let mut ch = Channel::new();
        ch.fold(18.0);
        ch.advance();
        ch.fold(22.0);
        ch.advance();
        ch.fold(15.0);
        let (lo, hi) = ch.min_max().unwrap();
        assert!((lo - 15.0).abs() < 1e-6);
        assert!((hi - 22.0).abs() < 1e-6);
    }

    #[test]
    fn slot_at_column_returns_oldest_to_newest() {
        let mut ch = Channel::new();
        // Slot 0 = first bucket = will become the oldest after we advance.
        ch.fold(1.0);
        ch.advance();
        ch.fold(2.0);
        ch.advance();
        ch.fold(3.0);
        // head now points at the slot holding 3.0; column 127 is newest.
        assert!((ch.slot_at_column(127).unwrap().min - 3.0).abs() < 1e-6);
        assert!((ch.slot_at_column(126).unwrap().min - 2.0).abs() < 1e-6);
        assert!((ch.slot_at_column(125).unwrap().min - 1.0).abs() < 1e-6);
        // Anything further back is still empty.
        assert!(ch.slot_at_column(124).is_none());
        assert!(ch.slot_at_column(0).is_none());
    }

    #[test]
    fn snapshot_roundtrip_preserves_buckets() {
        let mut h = TempHistory::new();
        // Two populated buckets per channel: oldest then newest.
        h.outdoor.fold(18.0); h.indoor.fold(21.0);
        h.outdoor.advance(); h.indoor.advance();
        h.outdoor.fold(19.5); h.indoor.fold(20.5);

        let json = h.snapshot_json(1_700_000_000);
        let mut h2 = TempHistory::new();
        assert!(h2.restore_from_snapshot(json.as_bytes()));

        // Newest (col 127) and the bucket before it (col 126) round-trip.
        assert!((h2.outdoor.slot_at_column(127).unwrap().min - 19.5).abs() < 0.01);
        assert!((h2.outdoor.slot_at_column(126).unwrap().min - 18.0).abs() < 0.01);
        assert!((h2.indoor.slot_at_column(127).unwrap().max - 20.5).abs() < 0.01);
        assert!((h2.indoor.slot_at_column(126).unwrap().max - 21.0).abs() < 0.01);
        // Empty older columns stay empty.
        assert!(h2.outdoor.slot_at_column(125).is_none());
    }

    #[test]
    fn restore_ignores_malformed_input() {
        let mut h = TempHistory::new();
        assert!(!h.restore_from_snapshot(b"not json at all"));
        assert!(!h.restore_from_snapshot(b"{}"));
    }
}
