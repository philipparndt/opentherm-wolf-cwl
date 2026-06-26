//! OLED display — 128x64 I2C, 6 pages with overlays.
//!
//! Driver selection is at compile time:
//!   * default (0.96" panels): SSD1306
//!   * `display-sh1106` feature (1.3" panels): SH1106
//! The two controllers share the same protocol surface but differ enough in
//! column offset and addressing-mode handling that mixing them produces noise.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle};
use esp_idf_svc::hal::i2c::I2cDriver;
use log::info;
use u8g2_fonts::fonts;
use u8g2_fonts::FontRenderer;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};

use crate::app_state::AppStateInner;
use crate::framebuffer::FrameBuffer;
use crate::history::Channel;
use crate::i18n::{Language, Strings, tr, level_name};

#[cfg(not(feature = "display-sh1106"))]
use ssd1306::mode::BufferedGraphicsMode;
#[cfg(not(feature = "display-sh1106"))]
use ssd1306::prelude::*;
#[cfg(not(feature = "display-sh1106"))]
use ssd1306::{I2CDisplayInterface, Ssd1306};

#[cfg(feature = "display-sh1106")]
use sh1106::{prelude::*, Builder};

type AppState = Arc<Mutex<AppStateInner>>;

#[cfg(not(feature = "display-sh1106"))]
type Disp = Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

#[cfg(feature = "display-sh1106")]
type Disp = GraphicsMode<I2cInterface<I2cDriver<'static>>>;

/// Clear the off-screen buffer. The two driver crates spell this differently
/// (`clear_buffer` on ssd1306, `clear` on sh1106 0.5) — wrap it once here so
/// the call sites stay clean.
#[inline]
fn clear_disp(d: &mut Disp) {
    #[cfg(not(feature = "display-sh1106"))]
    { d.clear_buffer(); }
    #[cfg(feature = "display-sh1106")]
    { d.clear(); }
}

pub const PAGE_COUNT: usize = 8;
const STANDBY_TIMEOUT_MS: u32 = 300_000;
const OVERLAY_TIMEOUT_MS: u32 = 10_000;
const EDIT_TIMEOUT_MS: u32 = 10_000;

// Font renderers matching C++ U8g2 fonts
const FONT_SMALL: FontRenderer = FontRenderer::new::<fonts::u8g2_font_helvR08_tr>();
const FONT_LARGE: FontRenderer = FontRenderer::new::<fonts::u8g2_font_helvB14_tr>();
const FONT_MEDIUM: FontRenderer = FontRenderer::new::<fonts::u8g2_font_helvB12_tr>();

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Home = 0, Bypass, TempIn,
    OutdoorHistory, IndoorHistory,
    Status, System, Settings,
}

impl Page {
    fn from_index(i: usize) -> Self {
        match i % PAGE_COUNT {
            0 => Self::Home, 1 => Self::Bypass, 2 => Self::TempIn,
            3 => Self::OutdoorHistory, 4 => Self::IndoorHistory,
            5 => Self::Status, 6 => Self::System, 7 => Self::Settings,
            _ => Self::Home,
        }
    }
    fn index(self) -> usize { self as usize }
}

/// Shared flag that gates whether `Display::update()` actually re-renders.
/// Set whenever something that affects the on-screen content changes — either
/// internally (page change, edit mode, overlay) or from another thread that
/// has touched `AppState` (OT sensor updates, MQTT commands). This is the
/// only way an external thread can request a render; cloning + bumping this
/// flag is far cheaper than locking the AppState mutex and is the reason we
/// no longer re-render ~7×/s when nothing has changed.
pub type DisplayDirty = Arc<AtomicBool>;

pub struct Display {
    display: Option<Disp>,
    fb: FrameBuffer,
    state: AppState,
    pub current_page: Page,
    pub edit_mode: bool,
    pub edit_vent_level: u8,
    pub edit_off_duration: bool,
    /// Index into `cwl_data::OFF_DURATIONS_MIN` for the currently-selected
    /// off duration in edit mode. Encoder rotation moves through the table
    /// (15 m → 30 m → … → 2 w).
    pub edit_off_idx: u8,
    pub standby: bool,
    last_activity_ms: u32,
    overlay_active: bool,
    overlay_header: String,
    overlay_message: String,
    overlay_start_ms: u32,
    edit_mode_start_ms: u32,
    dirty: DisplayDirty,
    // Tracks the current half of the filter-blink cycle (1 s on / 1 s off) so
    // update() can flip the dirty flag exactly when the phase boundary crosses
    // while filter_dirty is true, instead of forcing a redraw every frame.
    last_filter_blink_phase: bool,
}

impl Display {
    pub fn new(i2c: I2cDriver<'static>, state: AppState, dirty: DisplayDirty) -> Self {
        #[cfg(not(feature = "display-sh1106"))]
        let mut display = {
            let interface = I2CDisplayInterface::new(i2c);
            let rotation = if cfg!(feature = "display-rotate") {
                DisplayRotation::Rotate180
            } else {
                DisplayRotation::Rotate0
            };
            Ssd1306::new(interface, DisplaySize128x64, rotation)
                .into_buffered_graphics_mode()
        };

        #[cfg(feature = "display-sh1106")]
        let mut display: Disp = {
            let rotation = if cfg!(feature = "display-rotate") {
                DisplayRotation::Rotate180
            } else {
                DisplayRotation::Rotate0
            };
            Builder::new()
                .with_size(DisplaySize::Display128x64)
                .with_rotation(rotation)
                .connect_i2c(i2c)
                .into()
        };

        let driver_name = if cfg!(feature = "display-sh1106") { "SH1106" } else { "SSD1306" };
        let mut ok = false;
        for attempt in 1..=3 {
            if display.init().is_ok() {
                clear_disp(&mut display);
                display.flush().ok();
                ok = true;
                info!("Display: Initialized ({}, attempt {})", driver_name, attempt);
                break;
            }
            info!("Display: Init attempt {} failed, retrying...", attempt);
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        if !ok {
            info!("Display: Init failed after 3 attempts");
        }
        // Force one render on startup so the boot/home screen actually paints.
        dirty.store(true, Ordering::Relaxed);
        Self {
            display: if ok { Some(display) } else { None },
            fb: FrameBuffer::new(),
            state, current_page: Page::Home, edit_mode: false, edit_vent_level: 2,
            edit_off_duration: false, edit_off_idx: 3, standby: false, // default 1h (index 3)
            last_activity_ms: now(), overlay_active: false, overlay_header: String::new(),
            overlay_message: String::new(), overlay_start_ms: 0, edit_mode_start_ms: 0,
            dirty,
            last_filter_blink_phase: false,
        }
    }

    /// Mark the screen as needing a re-render. Internal mutations call this
    /// directly; external threads instead bump their clone of the shared
    /// `DisplayDirty` flag.
    fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn update(&mut self, _now_ms: u32) {
        let now_ms = now(); // Always use fresh timestamp to avoid race with wake()

        if self.edit_mode && now_ms.wrapping_sub(self.edit_mode_start_ms) > EDIT_TIMEOUT_MS {
            self.edit_mode = false;
            self.mark_dirty();
        }

        if let Some(ref mut display) = self.display {
            if !self.standby && !self.edit_mode
                && now_ms.saturating_sub(self.last_activity_ms) > STANDBY_TIMEOUT_MS {
                // Turn off display by clearing and flushing a blank screen
                clear_disp(display);
                display.flush().ok();
                self.standby = true;
                self.fb.clear();
                self.state.lock().unwrap().display_framebuffer = self.fb.buf;
                self.dirty.store(false, Ordering::Relaxed);
                return;
            }
            if self.standby && self.overlay_active {
                self.standby = false;
                self.mark_dirty();
            }
        }

        if self.standby { return; }
        if self.overlay_active && now_ms.wrapping_sub(self.overlay_start_ms) > OVERLAY_TIMEOUT_MS {
            self.overlay_active = false;
            self.mark_dirty();
        }

        // Filter-maintenance blink on the Home page: "Filter" shown for 1 s,
        // hidden for 1 s. Force a redraw exactly when the phase flips so we
        // don't burn cycles re-rendering between transitions.
        if self.current_page == Page::Home {
            let filter_dirty = self.state.lock().unwrap().cwl_data.filter_dirty;
            if filter_dirty {
                let phase = (now_ms / 1000) % 2 == 0;
                if phase != self.last_filter_blink_phase {
                    self.last_filter_blink_phase = phase;
                    self.mark_dirty();
                }
            }
        }

        // Skip the render path entirely if nothing has changed. The standby /
        // overlay / edit-mode timeout checks above still run on every call so
        // they remain time-driven even while we're idle.
        if !self.dirty.swap(false, Ordering::Relaxed) {
            return;
        }

        // Render once into the framebuffer (the source of truth for both the
        // OLED and the web mirror).
        self.fb.clear();
        {
            let st = self.state.lock().unwrap();
            Self::render_content(&mut self.fb, &st, self.current_page, self.edit_mode,
                self.edit_vent_level, self.edit_off_duration, self.edit_off_idx,
                self.overlay_active, &self.overlay_header, &self.overlay_message);
        }

        // Push the rendered framebuffer to the OLED. `draw_iter` yields every
        // pixel (on + off) so we don't need a separate `clear_disp` first.
        if let Some(ref mut d) = self.display {
            let _ = d.draw_iter(self.fb.pixels());
            d.flush().ok();
        }

        // Publish framebuffer for the web mirror (memcpy of 1 KB).
        self.state.lock().unwrap().display_framebuffer = self.fb.buf;
    }

    fn render_content(
        d: &mut impl DrawTarget<Color = BinaryColor>,
        st: &AppStateInner,
        page: Page, edit_mode: bool, edit_vent_level: u8,
        edit_off_duration: bool, edit_off_idx: u8,
        overlay_active: bool, overlay_header: &str, overlay_message: &str,
    ) {
        if overlay_active && !edit_mode {
            draw_overlay(d, overlay_header, overlay_message);
        } else if !st.cwl_data.connected && !edit_mode {
            draw_overlay(d, "CWL", tr(st.config.language).cwl_disconnected);
        } else {
            let lang = st.config.language;
            match page {
                Page::Home => draw_home(d, st, lang, edit_mode, edit_vent_level, edit_off_duration, edit_off_idx),
                Page::Bypass => draw_bypass(d, st, lang, edit_mode, edit_vent_level),
                Page::TempIn => draw_temp_in(d, st, lang),
                Page::OutdoorHistory => draw_outdoor_history(d, st, lang),
                Page::IndoorHistory => draw_indoor_history(d, st, lang),
                Page::Status => draw_status(d, st, lang),
                Page::System => draw_system(d, st, lang),
                Page::Settings => draw_settings(d, st, lang, edit_mode, edit_vent_level),
            }
        }

        #[cfg(feature = "simulate-ot")]
        {
            FONT_SMALL.render_aligned("SIM", Point::new(125, 0), VerticalPosition::Top,
                HorizontalAlignment::Right, FontColor::Transparent(BinaryColor::On), d).ok();
        }

        // Extreme-heat mode indicator — top-right, same row as the SIM tag.
        // The mode owns the ventilation level (overriding the schedule) whenever
        // it's enabled, so surface it here. Sits left of SIM in simulator builds
        // so the two don't overlap.
        if st.config.extreme_heat_enabled {
            let x = if cfg!(feature = "simulate-ot") { 105 } else { 125 };
            FONT_SMALL.render_aligned("EH", Point::new(x, 0), VerticalPosition::Top,
                HorizontalAlignment::Right, FontColor::Transparent(BinaryColor::On), d).ok();
        }

        // Dots indicator
        if edit_mode && edit_off_duration {
            // Off hours selection — no dots
        } else if edit_mode && (page == Page::Home) {
            // Level selection — 5 mode dots (Off/Reduced/Normal/Party/Schedule)
            let count = 5i32;
            let start = (128 - (count - 1) * 7) / 2;
            for i in 0..count {
                let style = if i as u8 == edit_vent_level {
                    PrimitiveStyle::with_fill(BinaryColor::On)
                } else {
                    PrimitiveStyle::with_stroke(BinaryColor::On, 1)
                };
                Circle::new(Point::new(start + i * 7 - 2, 59), 5).into_styled(style).draw(d).ok();
            }
        } else if edit_mode && (page == Page::Bypass || page == Page::Settings) {
            // Toggle — 2 mode dots
            let count = 2i32;
            let start = (128 - (count - 1) * 7) / 2;
            for i in 0..count {
                let style = if i as u8 == edit_vent_level {
                    PrimitiveStyle::with_fill(BinaryColor::On)
                } else {
                    PrimitiveStyle::with_stroke(BinaryColor::On, 1)
                };
                Circle::new(Point::new(start + i * 7 - 2, 59), 5).into_styled(style).draw(d).ok();
            }
        } else {
            // Normal page dots
            let start = (128 - (PAGE_COUNT as i32 - 1) * 7) / 2;
            for i in 0..PAGE_COUNT {
                let style = if i == page.index() {
                    PrimitiveStyle::with_fill(BinaryColor::On)
                } else {
                    PrimitiveStyle::with_stroke(BinaryColor::On, 1)
                };
                Circle::new(Point::new(start + i as i32 * 7 - 2, 59), 5).into_styled(style).draw(d).ok();
            }
        }
    }

    #[allow(dead_code)]
    fn render_to_target(&self, fb: &mut FrameBuffer, _now_ms: u32) {
        fb.clear();
        let st = self.state.lock().unwrap();
        Self::render_content(fb, &st, self.current_page, self.edit_mode,
            self.edit_vent_level, self.edit_off_duration, self.edit_off_idx,
            self.overlay_active, &self.overlay_header, &self.overlay_message);
        drop(st);
        self.state.lock().unwrap().display_framebuffer = fb.buf;
    }

    pub fn wake(&mut self) -> bool {
        self.last_activity_ms = now();
        if self.overlay_active {
            self.overlay_active = false;
            self.mark_dirty();
        }
        if self.standby {
            self.standby = false;
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn show_ip(&mut self, ip: &str) {
        let lang = self.state.lock().unwrap().config.language;
        self.overlay_header = tr(lang).connected.into();
        self.overlay_message = ip.into();
        self.overlay_start_ms = now();
        self.overlay_active = true;
        self.last_activity_ms = self.overlay_start_ms;
        self.mark_dirty();
    }

    pub fn show_disconnected(&mut self) {
        let lang = self.state.lock().unwrap().config.language;
        let s = tr(lang);
        self.overlay_header = s.disconnected.into();
        self.overlay_message = s.no_network.into();
        self.overlay_start_ms = now();
        self.overlay_active = true;
        self.last_activity_ms = self.overlay_start_ms;
        self.mark_dirty();
    }

    pub fn next_page(&mut self) {
        self.last_activity_ms = now();
        let next = self.current_page.index() + 1;
        if next < PAGE_COUNT {
            self.current_page = Page::from_index(next);
            self.mark_dirty();
        }
    }
    pub fn prev_page(&mut self) {
        self.last_activity_ms = now();
        let curr = self.current_page.index();
        if curr > 0 {
            self.current_page = Page::from_index(curr - 1);
            self.mark_dirty();
        }
    }

    /// Show boot screen
    pub fn boot_screen(&mut self) {
        let display = match &mut self.display { Some(d) => d, None => return };
        let lang = self.state.lock().unwrap().config.language;
        clear_disp(display);
        FONT_LARGE.render_aligned("Wolf CWL", Point::new(64, 20),
            VerticalPosition::Top, HorizontalAlignment::Center,
            FontColor::Transparent(BinaryColor::On), display).ok();
        FONT_SMALL.render_aligned(tr(lang).connecting, Point::new(64, 44),
            VerticalPosition::Top, HorizontalAlignment::Center,
            FontColor::Transparent(BinaryColor::On), display).ok();
        display.flush().ok();
    }

    /// Enter edit mode on the current page
    pub fn enter_edit_mode(&mut self) {
        if self.current_page == Page::Home {
            let st = self.state.lock().unwrap();
            self.edit_vent_level = if st.timed_off_active { 0 } else { st.cwl_data.ventilation_level };
            drop(st);
            self.edit_off_duration = false;
            self.edit_off_idx = 3; // default 1h
            self.edit_mode = true;
            self.edit_mode_start_ms = now();
            self.mark_dirty();
        } else if self.current_page == Page::Bypass {
            let st = self.state.lock().unwrap();
            self.edit_vent_level = if st.requested_bypass_open { 1 } else { 0 };
            drop(st);
            self.edit_mode = true;
            self.edit_mode_start_ms = now();
            self.mark_dirty();
        } else if self.current_page == Page::Settings {
            let st = self.state.lock().unwrap();
            self.edit_vent_level = st.config.language as u8;
            drop(st);
            self.edit_mode = true;
            self.edit_mode_start_ms = now();
            self.mark_dirty();
        }
    }

    /// Exit edit mode, optionally applying the change.
    /// Returns true if edit mode was fully exited, false if entering sub-stage.
    pub fn exit_edit_mode(&mut self, apply: bool) -> bool {
        if apply && self.current_page == Page::Home {
            if self.edit_off_duration {
                // Stage 2 confirmed → activate timed off
                let idx = (self.edit_off_idx as usize).min(crate::cwl_data::OFF_DURATIONS_MIN.len() - 1);
                let minutes = crate::cwl_data::OFF_DURATIONS_MIN[idx];
                let mut st = self.state.lock().unwrap();
                st.timed_off_request = Some(minutes);
                st.display_wake_requested = true;
            } else if self.edit_vent_level == 4 {
                // Selected Schedule → clear override, return to schedule control
                let mut st = self.state.lock().unwrap();
                if st.timed_off_active {
                    st.cancel_timed_off = true;
                }
                st.schedule_override = false;
            } else if self.edit_vent_level == 0 {
                // Selected Off → enter duration sub-stage
                self.edit_off_duration = true;
                self.edit_off_idx = 3; // default 1h
                self.edit_mode_start_ms = now();
                return false; // Don't exit edit mode
            } else {
                // Non-off level (Reduced/Normal/Party)
                let mut st = self.state.lock().unwrap();
                if st.timed_off_active {
                    st.cancel_timed_off = true;
                }
                st.requested_vent_level = self.edit_vent_level;
                st.config.ventilation_level = self.edit_vent_level;
                st.schedule_override = true;
                st.initial_level_known = true;
                st.push_history_marker_now(self.edit_vent_level, crate::app_state::Reason::Manual);
            }
        } else if apply && self.current_page == Page::Bypass {
            let open = self.edit_vent_level != 0;
            let mut st = self.state.lock().unwrap();
            st.set_bypass_open(open);
            st.config.bypass_open = open;
            st.persist_config = true;
        } else if apply && self.current_page == Page::Settings {
            let mut st = self.state.lock().unwrap();
            st.config.language = Language::from_u8(self.edit_vent_level);
            st.persist_config = true;
        }
        self.edit_mode = false;
        self.edit_off_duration = false;
        self.mark_dirty();
        true
    }

    /// Adjust edit value by delta
    pub fn adjust_edit_value(&mut self, delta: i32) {
        if !self.edit_mode { return; }
        self.edit_mode_start_ms = now();
        self.mark_dirty();
        if self.current_page == Page::Home {
            if self.edit_off_duration {
                // Step through the discrete OFF_DURATIONS_MIN table.
                let max = crate::cwl_data::OFF_DURATIONS_MIN.len() as i32 - 1;
                let new_idx = self.edit_off_idx as i32 + delta;
                if new_idx < 0 {
                    // Rotate back past the shortest entry → exit duration
                    // sub-stage and reselect Schedule from the level picker.
                    self.edit_off_duration = false;
                    self.edit_vent_level = 4; // Schedule
                } else {
                    self.edit_off_idx = new_idx.min(max) as u8;
                }
            } else {
                // Cycling through levels 0-4 (Off/Reduced/Normal/Party/Schedule)
                let new_level = (self.edit_vent_level as i32 + delta).rem_euclid(5) as u8;
                self.edit_vent_level = new_level;
            }
        } else if self.current_page == Page::Bypass {
            // Toggle between 0 (winter) and 1 (summer)
            self.edit_vent_level = if self.edit_vent_level == 0 { 1 } else { 0 };
        } else if self.current_page == Page::Settings {
            // Toggle between 0 (English) and 1 (German)
            self.edit_vent_level = if self.edit_vent_level == 0 { 1 } else { 0 };
        }
    }
}

fn now() -> u32 { unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u32 } }

/// Draw header with underline — matches C++ drawHeader()
fn draw_header(d: &mut impl DrawTarget<Color = BinaryColor>, title: &str) {
    FONT_SMALL.render_aligned(title, Point::new(0, 0),
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        FontColor::Transparent(BinaryColor::On), d).ok();
    Line::new(Point::new(0, 11), Point::new(125, 11))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(d).ok();
}

/// Draw large centered text — matches C++ drawLevelCentered()
fn draw_centered(d: &mut impl DrawTarget<Color = BinaryColor>, text: &str, y: i32) {
    // Try large font first, fall back to medium if too wide
    let w = FONT_LARGE.get_rendered_dimensions(text, Point::zero(), VerticalPosition::Top)
        .ok()
        .and_then(|r| r.bounding_box)
        .map(|bb| bb.size.width as i32)
        .unwrap_or(0);
    if w <= 124 {
        FONT_LARGE.render_aligned(text, Point::new(64, y),
            VerticalPosition::Top, HorizontalAlignment::Center,
            FontColor::Transparent(BinaryColor::On), d).ok();
    } else {
        FONT_MEDIUM.render_aligned(text, Point::new(64, y),
            VerticalPosition::Top, HorizontalAlignment::Center,
            FontColor::Transparent(BinaryColor::On), d).ok();
    }
}

fn draw_large_left(d: &mut impl DrawTarget<Color = BinaryColor>, text: &str, x: i32, y: i32) {
    FONT_LARGE.render_aligned(text, Point::new(x, y),
        VerticalPosition::Top, HorizontalAlignment::Left,
        FontColor::Transparent(BinaryColor::On), d).ok();
}

fn draw_small_centered(d: &mut impl DrawTarget<Color = BinaryColor>, text: &str, y: i32) {
    FONT_SMALL.render_aligned(text, Point::new(64, y),
        VerticalPosition::Top, HorizontalAlignment::Center,
        FontColor::Transparent(BinaryColor::On), d).ok();
}

fn draw_small(d: &mut impl DrawTarget<Color = BinaryColor>, text: &str, x: i32, y: i32) {
    FONT_SMALL.render_aligned(text, Point::new(x, y),
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        FontColor::Transparent(BinaryColor::On), d).ok();
}

fn draw_overlay(d: &mut impl DrawTarget<Color = BinaryColor>, header: &str, message: &str) {
    FONT_SMALL.render_aligned(header, Point::new(64, 18),
        VerticalPosition::Top,
        HorizontalAlignment::Center,
        FontColor::Transparent(BinaryColor::On), d).ok();
    draw_centered(d, message, 30);
}

fn draw_home(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language, edit_mode: bool, edit_level: u8, edit_off_duration: bool, edit_off_idx: u8) {
    let s = tr(lang);
    // Filter-maintenance: when the CWL is asking for service, blink "Filter"
    // with a 2 s period (1 s on / 1 s off) in the level slot — replacing the
    // "Normal" / "Party" / etc. text — while keeping the regular header and
    // the m³/h info line. Edit-mode is unaffected so an interaction in
    // progress isn't blocked.
    let filter_blink_on = st.cwl_data.filter_dirty && (now() / 1000) % 2 == 0;
    if edit_mode && edit_off_duration {
        // Stage 2: selecting off duration
        draw_header(d, s.off_duration);
        let durations = crate::cwl_data::OFF_DURATIONS_MIN;
        let idx = (edit_off_idx as usize).min(durations.len() - 1);
        let buf = crate::cwl_data::format_off_duration(durations[idx]);
        draw_centered(d, &buf, 22);
        draw_small_centered(d, s.hint_rotate_hours, 42);
    } else if edit_mode {
        // Stage 1: selecting level
        draw_header(d, s.set_level);
        draw_centered(d, level_name(lang, edit_level), 22);
        draw_small_centered(d, s.hint_rotate_adjust, 42);
    } else if st.timed_off_active {
        // Timed off countdown
        draw_header(d, s.manual);
        draw_centered(d, s.level_off, 18);
        let info = format!("{} {}",
            s.resumes_in,
            crate::cwl_data::format_off_duration(st.timed_off_remaining_min.min(u16::MAX as u32) as u16));
        draw_small_centered(d, &info, 42);
    } else {
        let header = if st.schedule_override { s.manual } else if st.schedule_active { s.scheduled } else { s.ventilation };
        draw_header(d, header);
        if filter_blink_on {
            draw_centered(d, "Filter", 18);
        } else if st.requested_vent_level != st.cwl_data.ventilation_level {
            draw_small(d, ">", 0, 22);
            draw_centered(d, level_name(lang, st.requested_vent_level), 18);
        } else {
            draw_centered(d, level_name(lang, st.cwl_data.ventilation_level), 18);
        }
        let mode = if st.requested_bypass_open { s.summer } else { s.winter };
        // Prefer the actual outlet volume (TSP 52,53) since it's far more
        // useful than a relative %; fall back to % until the TSP scan reaches
        // those registers (~a few minutes after boot).
        let info = if st.cwl_data.current_volume > 0 {
            format!("{} m\u{00B3}/h  {}", st.cwl_data.current_volume, mode)
        } else {
            format!("{}%  {}", st.cwl_data.relative_ventilation, mode)
        };
        draw_small_centered(d, &info, 42);
    }
}

fn draw_bypass(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language, edit_mode: bool, edit_vent_level: u8) {
    let s = tr(lang);
    if edit_mode {
        draw_header(d, s.set_mode);
        draw_centered(d, if edit_vent_level != 0 { s.summer } else { s.winter }, 22);
        draw_small_centered(d, s.hint_rotate_toggle, 42);
    } else {
        draw_header(d, s.summer_mode);
        draw_centered(d, if st.requested_bypass_open { s.active } else { s.inactive }, 18);
        let desc = if st.requested_bypass_open { s.bypass_open_desc } else { s.heat_recovery_desc };
        draw_small_centered(d, desc, 42);
    }
}

fn draw_temp_value(d: &mut impl DrawTarget<Color = BinaryColor>, label: &str, temp: f32, y: i32) {
    // Label in small font, value in large font — same baseline
    // helvR08 is ~8px tall, helvB14 is ~14px tall. To share baseline,
    // place small text higher so bottoms align: large at y, small at y + (14 - 8) = y + 6
    draw_small(d, label, 0, y + 4);
    let val = format!("{:.1} C", temp);
    // Label "Supply:" is ~48px wide, place value after it
    draw_large_left(d, &val, 52, y);
}

/// Intake: temperatures entering the heat exchanger
/// - Supply inlet (ID 80): fresh air from outside
/// - Exhaust inlet (ID 82): stale air from the house
fn draw_temp_in(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language) {
    let s = tr(lang);
    draw_header(d, s.intake);
    draw_temp_value(d, s.supply, st.cwl_data.supply_temp, 18);
    draw_temp_value(d, s.exhaust, st.cwl_data.exhaust_temp, 38);
}

fn draw_status(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language) {
    let s = tr(lang);
    draw_header(d, s.status);
    let s2 = format!("{} {}", s.filter, if st.cwl_data.filter_dirty { s.replace } else { s.ok });
    draw_small(d, &s2, 0, 14);
    let s3 = format!("{} {}", s.mode, if st.requested_bypass_open { s.summer } else { s.winter });
    draw_small(d, &s3, 0, 25);
    if st.cwl_data.tsp_valid[52] {
        let s4 = format!("{} {} m3/h", s.airflow, st.cwl_data.current_volume);
        draw_small(d, &s4, 0, 36);
    }
}

fn draw_settings(d: &mut impl DrawTarget<Color = BinaryColor>, _st: &AppStateInner, lang: Language, edit_mode: bool, edit_level: u8) {
    let s = tr(lang);
    draw_header(d, s.settings);
    if edit_mode {
        let name = if edit_level == 0 { s.english } else { s.deutsch };
        draw_centered(d, name, 22);
        draw_small_centered(d, s.hint_rotate_toggle, 42);
    } else {
        draw_small(d, s.language_label, 0, 18);
        let current = if lang == Language::En { s.english } else { s.deutsch };
        draw_centered(d, current, 30);
    }
}

fn draw_temp_chart(
    d: &mut impl DrawTarget<Color = BinaryColor>,
    channel: &Channel,
    strings: &Strings,
    show_zero_axis: bool,
) {
    let chart_top = 24i32;
    let chart_bottom = 56i32;

    let (lo, hi) = match channel.min_max() {
        Some(v) => v,
        None => {
            // Empty placeholders for min/max strip; centred hint in chart area.
            let strip = format!("{} --   {} --", strings.min_label, strings.max_label);
            draw_small_centered(d, &strip, 14);
            draw_small_centered(d, strings.history_empty, 36);
            return;
        }
    };

    let strip = format!(
        "{} {:.1}{}   {} {:.1}{}",
        strings.min_label, lo, strings.celsius_unit,
        strings.max_label, hi, strings.celsius_unit,
    );
    draw_small_centered(d, &strip, 14);

    let raw_range = hi - lo;
    let range = raw_range.max(0.5);
    let pad = (range - raw_range) / 2.0;
    let mut eff_lo = lo - pad;
    let mut eff_hi = hi + pad;
    // For the gain chart, keep 0 °C inside the visible range so the
    // zero axis is meaningful even when the data sits entirely above
    // or below it.
    if show_zero_axis {
        if eff_lo > 0.0 { eff_lo = 0.0; }
        if eff_hi < 0.0 { eff_hi = 0.0; }
    }
    let span = eff_hi - eff_lo;

    let scale = |t: f32| -> i32 {
        let frac = ((t - eff_lo) / span).clamp(0.0, 1.0);
        let y = chart_bottom as f32 - frac * (chart_bottom - chart_top) as f32;
        y.round() as i32
    };

    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    // Dotted 0-axis behind the data — every other pixel so the data lines
    // remain readable where they cross the axis.
    if show_zero_axis {
        let y0 = scale(0.0);
        let pixels = (0..128i32)
            .filter(|x| x % 2 == 0)
            .map(move |x| Pixel(Point::new(x, y0), BinaryColor::On));
        let _ = d.draw_iter(pixels);
    }

    for col in 0..128usize {
        // The history is finer than 128 px now; aggregate each column's span.
        if let Some(b) = channel.aggregated_column(col, 128) {
            let y_top = scale(b.max);
            let y_bottom = scale(b.min);
            let x = col as i32;
            if y_top == y_bottom {
                let _ = d.draw_iter(core::iter::once(Pixel(Point::new(x, y_top), BinaryColor::On)));
            } else {
                Line::new(Point::new(x, y_top), Point::new(x, y_bottom))
                    .into_styled(stroke).draw(d).ok();
            }
        }
    }
}

fn draw_outdoor_history(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language) {
    let s = tr(lang);
    draw_header(d, s.outdoor_24h);
    draw_temp_chart(d, &st.temp_history.outdoor, s, false);
}

fn draw_indoor_history(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language) {
    let s = tr(lang);
    draw_header(d, s.indoor_24h);
    draw_temp_chart(d, &st.temp_history.indoor, s, false);
}

fn draw_system(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, lang: Language) {
    let s = tr(lang);
    draw_header(d, s.system);
    let net_str = if st.network_connected {
        match &st.ip_address {
            Some(ip) => format!("Net: {}", ip),
            None => s.net_connected.into(),
        }
    } else {
        s.net_disconnected.into()
    };
    draw_small(d, &net_str, 0, 14);
    draw_small(d, if st.mqtt_connected { s.mqtt_online } else { s.mqtt_offline }, 0, 25);
    let uptime_s = unsafe { esp_idf_svc::sys::esp_timer_get_time() / 1_000_000 } as u64;
    let up_str = format!("{} {}h {}m", s.uptime_prefix, uptime_s / 3600, (uptime_s % 3600) / 60);
    draw_small(d, &up_str, 0, 35);
    let reason = format!("{} {}", s.boot_prefix, crate::watchdog::reboot_reason());
    draw_small(d, &reason, 0, 46);
}
