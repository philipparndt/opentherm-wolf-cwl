//! OLED display — 128x64 I2C, SH1106, 6 pages with overlays.

use std::sync::{Arc, Mutex};

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle};
use esp_idf_svc::hal::i2c::I2cDriver;
use log::info;
use sh1106::prelude::*;
use sh1106::Builder;
use u8g2_fonts::fonts;
use u8g2_fonts::FontRenderer;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};

use crate::app_state::AppStateInner;
use crate::cwl_data::ventilation_level_name;
use crate::framebuffer::FrameBuffer;


type AppState = Arc<Mutex<AppStateInner>>;
type Disp = GraphicsMode<sh1106::interface::I2cInterface<I2cDriver<'static>>>;

pub const PAGE_COUNT: usize = 6;
const STANDBY_TIMEOUT_MS: u32 = 300_000;
const OVERLAY_TIMEOUT_MS: u32 = 10_000;
const EDIT_TIMEOUT_MS: u32 = 10_000;

// Font renderers matching C++ U8g2 fonts
const FONT_SMALL: FontRenderer = FontRenderer::new::<fonts::u8g2_font_helvR08_tr>();
const FONT_LARGE: FontRenderer = FontRenderer::new::<fonts::u8g2_font_helvB14_tr>();
const FONT_MEDIUM: FontRenderer = FontRenderer::new::<fonts::u8g2_font_helvB12_tr>();

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Home = 0, Bypass, TempIn, TempOut, Status, System,
}

impl Page {
    fn from_index(i: usize) -> Self {
        match i % PAGE_COUNT { 0 => Self::Home, 1 => Self::Bypass, 2 => Self::TempIn, 3 => Self::TempOut, 4 => Self::Status, 5 => Self::System, _ => Self::Home }
    }
    fn index(self) -> usize { self as usize }
}

pub struct Display {
    display: Option<Disp>,
    fb: FrameBuffer,
    state: AppState,
    pub current_page: Page,
    pub edit_mode: bool,
    pub edit_vent_level: u8,
    pub edit_off_duration: bool,
    pub edit_off_hours: u8,
    pub standby: bool,
    last_activity_ms: u32,
    overlay_active: bool,
    overlay_header: String,
    overlay_message: String,
    overlay_start_ms: u32,
    edit_mode_start_ms: u32,
}

impl Display {
    pub fn new(i2c: I2cDriver<'static>, state: AppState) -> Self {
        let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();
        let ok = display.init().is_ok();
        if ok {
            display.flush().ok();
            info!("Display: Initialized (SH1106)");
        } else {
            info!("Display: Init failed");
        }
        Self {
            display: if ok { Some(display) } else { None },
            fb: FrameBuffer::new(),
            state, current_page: Page::Home, edit_mode: false, edit_vent_level: 2,
            edit_off_duration: false, edit_off_hours: 1, standby: false,
            last_activity_ms: 0, overlay_active: false, overlay_header: String::new(),
            overlay_message: String::new(), overlay_start_ms: 0, edit_mode_start_ms: 0,
        }
    }

    pub fn update(&mut self, _now_ms: u32) {
        let now_ms = now(); // Always use fresh timestamp to avoid race with wake()

        if self.edit_mode && now_ms.wrapping_sub(self.edit_mode_start_ms) > EDIT_TIMEOUT_MS {
            self.edit_mode = false;
        }

        if let Some(ref mut display) = self.display {
            if !self.standby && !self.edit_mode
                && now_ms.saturating_sub(self.last_activity_ms) > STANDBY_TIMEOUT_MS {
                // Turn off display by clearing and flushing a blank screen
                display.clear();
                display.flush().ok();
                self.standby = true;
                self.fb.clear();
                self.state.lock().unwrap().display_framebuffer = self.fb.buf;
                return;
            }
            if self.standby && self.overlay_active {
                self.standby = false;
                // Display will be redrawn on next frame
            }
        }

        if self.standby { return; }
        if self.overlay_active && now_ms.wrapping_sub(self.overlay_start_ms) > OVERLAY_TIMEOUT_MS {
            self.overlay_active = false;
        }

        // Draw directly to OLED (single render, fast)
        let display = match &mut self.display {
            Some(d) => d,
            None => {
                // No OLED — draw to framebuffer only (for web mirror)
                self.fb.clear();
                {
                    let st = self.state.lock().unwrap();
                    Self::render_content(&mut self.fb, &st, self.current_page, self.edit_mode,
                        self.edit_vent_level, self.edit_off_duration, self.edit_off_hours,
                        self.overlay_active, &self.overlay_header, &self.overlay_message);
                }
                self.state.lock().unwrap().display_framebuffer = self.fb.buf;
                return;
            }
        };

        display.clear();
        {
            let st = self.state.lock().unwrap();
            Self::render_content(display, &st, self.current_page, self.edit_mode,
                self.edit_vent_level, self.edit_off_duration, self.edit_off_hours,
                self.overlay_active, &self.overlay_header, &self.overlay_message);
        }
        display.flush().ok();

        // Update web mirror framebuffer every frame (just a memcpy — fast)
        self.fb.clear();
        {
            let st = self.state.lock().unwrap();
            Self::render_content(&mut self.fb, &st, self.current_page, self.edit_mode,
                self.edit_vent_level, self.edit_off_duration, self.edit_off_hours,
                self.overlay_active, &self.overlay_header, &self.overlay_message);
        }
        self.state.lock().unwrap().display_framebuffer = self.fb.buf;
    }

    fn render_content(
        d: &mut impl DrawTarget<Color = BinaryColor>,
        st: &AppStateInner,
        page: Page, edit_mode: bool, edit_vent_level: u8,
        edit_off_duration: bool, edit_off_hours: u8,
        overlay_active: bool, overlay_header: &str, overlay_message: &str,
    ) {
        if overlay_active && !edit_mode {
            draw_overlay(d, overlay_header, overlay_message);
        } else if !st.cwl_data.connected && !edit_mode {
            draw_overlay(d, "CWL", "Disconnected");
        } else {
            match page {
                Page::Home => draw_home(d, st, edit_mode, edit_vent_level, edit_off_duration, edit_off_hours),
                Page::Bypass => draw_bypass(d, st, edit_mode, edit_vent_level),
                Page::TempIn => draw_temp_in(d, st),
                Page::TempOut => draw_temp_out(d, st),
                Page::Status => draw_status(d, st),
                Page::System => draw_system(d, st),
            }
        }

        #[cfg(feature = "simulate-ot")]
        {
            FONT_SMALL.render_aligned("SIM", Point::new(125, 0), VerticalPosition::Top,
                HorizontalAlignment::Right, FontColor::Transparent(BinaryColor::On), d).ok();
        }

        // Dots indicator
        if edit_mode && edit_off_duration {
            // Off hours selection — no dots
        } else if edit_mode && (page == Page::Home) {
            // Level selection — 4 mode dots (Off/Reduced/Normal/Party)
            let count = 4i32;
            let start = (128 - (count - 1) * 7) / 2;
            for i in 0..count {
                let style = if i as u8 == edit_vent_level {
                    PrimitiveStyle::with_fill(BinaryColor::On)
                } else {
                    PrimitiveStyle::with_stroke(BinaryColor::On, 1)
                };
                Circle::new(Point::new(start + i * 7 - 2, 59), 5).into_styled(style).draw(d).ok();
            }
        } else if edit_mode && (page == Page::Bypass) {
            // Bypass toggle — 2 mode dots (Winter/Summer)
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
            self.edit_vent_level, self.edit_off_duration, self.edit_off_hours,
            self.overlay_active, &self.overlay_header, &self.overlay_message);
        drop(st);
        self.state.lock().unwrap().display_framebuffer = fb.buf;
    }

    pub fn wake(&mut self) -> bool {
        self.last_activity_ms = now();
        self.overlay_active = false;
        if self.standby {
            self.standby = false;
            // Next update() call will redraw the display
            return true;
        }
        false
    }

    pub fn show_ip(&mut self, ip: &str) {
        self.overlay_header = "Connected".into();
        self.overlay_message = ip.into();
        self.overlay_start_ms = now();
        self.overlay_active = true;
        self.last_activity_ms = self.overlay_start_ms;
    }

    pub fn show_disconnected(&mut self) {
        self.overlay_header = "Disconnected".into();
        self.overlay_message = "No network".into();
        self.overlay_start_ms = now();
        self.overlay_active = true;
        self.last_activity_ms = self.overlay_start_ms;
    }

    pub fn next_page(&mut self) {
        self.current_page = Page::from_index(self.current_page.index() + 1);
        self.last_activity_ms = now();
    }
    pub fn prev_page(&mut self) {
        self.current_page = Page::from_index(self.current_page.index() + PAGE_COUNT - 1);
        self.last_activity_ms = now();
    }

    /// Show boot screen
    pub fn boot_screen(&mut self) {
        let display = match &mut self.display { Some(d) => d, None => return };
        display.clear();
        FONT_LARGE.render_aligned("Wolf CWL", Point::new(64, 20),
            VerticalPosition::Top, HorizontalAlignment::Center,
            FontColor::Transparent(BinaryColor::On), display).ok();
        FONT_SMALL.render_aligned("Connecting...", Point::new(64, 44),
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
            self.edit_off_hours = 1;
            self.edit_mode = true;
            self.edit_mode_start_ms = now();
        } else if self.current_page == Page::Bypass {
            let st = self.state.lock().unwrap();
            self.edit_vent_level = if st.requested_bypass_open { 1 } else { 0 };
            drop(st);
            self.edit_mode = true;
            self.edit_mode_start_ms = now();
        }
    }

    /// Exit edit mode, optionally applying the change.
    /// Returns true if edit mode was fully exited, false if entering sub-stage.
    pub fn exit_edit_mode(&mut self, apply: bool) -> bool {
        if apply && self.current_page == Page::Home {
            if self.edit_off_duration {
                // Stage 2 confirmed → activate timed off
                let mut st = self.state.lock().unwrap();
                st.timed_off_request = Some(self.edit_off_hours);
                st.display_wake_requested = true;
            } else if self.edit_vent_level == 0 {
                // Selected Off → enter duration sub-stage
                self.edit_off_duration = true;
                self.edit_off_hours = 1;
                self.edit_mode_start_ms = now();
                return false; // Don't exit edit mode
            } else {
                // Non-off level
                let mut st = self.state.lock().unwrap();
                if st.timed_off_active {
                    st.cancel_timed_off = true;
                }
                st.requested_vent_level = self.edit_vent_level;
                st.config.ventilation_level = self.edit_vent_level;
                st.schedule_override = true;
            }
        } else if apply && self.current_page == Page::Bypass {
            let open = self.edit_vent_level != 0;
            let mut st = self.state.lock().unwrap();
            st.requested_bypass_open = open;
            st.config.bypass_open = open;
        }
        self.edit_mode = false;
        self.edit_off_duration = false;
        true
    }

    /// Adjust edit value by delta
    pub fn adjust_edit_value(&mut self, delta: i32) {
        if !self.edit_mode { return; }
        self.edit_mode_start_ms = now();
        if self.current_page == Page::Home {
            if self.edit_off_duration {
                // Adjusting off duration hours
                let new_hours = self.edit_off_hours as i32 + delta;
                if new_hours < 1 {
                    // Rotate back past 1h → exit duration sub-stage, go to Party
                    self.edit_off_duration = false;
                    self.edit_vent_level = 3; // Party
                } else if new_hours > 99 {
                    self.edit_off_hours = 99;
                } else {
                    self.edit_off_hours = new_hours as u8;
                }
            } else {
                // Cycling through levels 0-3
                let new_level = (self.edit_vent_level as i32 + delta).rem_euclid(4) as u8;
                self.edit_vent_level = new_level;
            }
        } else if self.current_page == Page::Bypass {
            // Toggle between 0 (winter) and 1 (summer)
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

fn draw_home(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, edit_mode: bool, edit_level: u8, edit_off_duration: bool, edit_off_hours: u8) {
    if edit_mode && edit_off_duration {
        // Stage 2: selecting off duration
        draw_header(d, "Off Duration");
        let buf = format!("{}h", edit_off_hours);
        draw_centered(d, &buf, 22);
        draw_small_centered(d, "rotate: hours  press: ok", 42);
    } else if edit_mode {
        // Stage 1: selecting level
        draw_header(d, "Set Level");
        draw_centered(d, ventilation_level_name(edit_level), 22);
        draw_small_centered(d, "rotate: adjust  press: ok", 42);
    } else if st.timed_off_active {
        // Timed off countdown
        draw_header(d, "Ventilation");
        draw_centered(d, "Off", 18);
        let rem = st.timed_off_remaining_min;
        let info = format!("Resumes in {}h {}m", rem / 60, rem % 60);
        draw_small_centered(d, &info, 42);
    } else {
        draw_header(d, "Ventilation");
        draw_centered(d, ventilation_level_name(st.cwl_data.ventilation_level), 18);
        let ind = if st.schedule_override { " M" } else if st.schedule_active { " S" } else { "" };
        let mode = if st.requested_bypass_open { "Summer" } else { "Winter" };
        let info = format!("{}%{}  {}", st.cwl_data.relative_ventilation, ind, mode);
        draw_small_centered(d, &info, 42);
    }
}

fn draw_bypass(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner, edit_mode: bool, edit_vent_level: u8) {
    if edit_mode {
        draw_header(d, "Set Mode");
        draw_centered(d, if edit_vent_level != 0 { "Summer" } else { "Winter" }, 22);
        draw_small_centered(d, "rotate: toggle  press: ok", 42);
    } else {
        draw_header(d, "Summer Mode");
        draw_centered(d, if st.requested_bypass_open { "Active" } else { "Inactive" }, 18);
        let desc = if st.requested_bypass_open { "Bypass open - free cooling" } else { "Heat recovery active" };
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

fn draw_temp_in(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner) {
    draw_header(d, "Intake");
    draw_temp_value(d, "Supply:", st.cwl_data.supply_inlet_temp, 18);
    draw_temp_value(d, "Exhaust:", st.cwl_data.exhaust_inlet_temp, 38);
}

fn draw_temp_out(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner) {
    draw_header(d, "Outlet");
    let mut y = 18i32;
    if st.cwl_data.supports_id81 {
        draw_temp_value(d, "Supply:", st.cwl_data.supply_outlet_temp, y);
        y += 20;
    }
    if st.cwl_data.supports_id83 {
        draw_temp_value(d, "Exhaust:", st.cwl_data.exhaust_outlet_temp, y);
    }
}

fn draw_status(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner) {
    draw_header(d, "Status");
    let s1 = format!("Fault:   {}", if st.cwl_data.fault { "YES" } else { "No" });
    draw_small(d, &s1, 0, 14);
    let s2 = format!("Filter:  {}", if st.cwl_data.filter_dirty { "REPLACE" } else { "OK" });
    draw_small(d, &s2, 0, 25);
    let s3 = format!("Mode:    {}", if st.requested_bypass_open { "Summer" } else { "Winter" });
    draw_small(d, &s3, 0, 36);
    if st.cwl_data.tsp_valid[52] {
        let s4 = format!("Airflow: {} m3/h", st.cwl_data.current_volume);
        draw_small(d, &s4, 0, 43);
    }
}

fn draw_system(d: &mut impl DrawTarget<Color = BinaryColor>, st: &AppStateInner) {
    draw_header(d, "System");
    let net_str = if st.network_connected {
        match &st.ip_address {
            Some(ip) => format!("Net: {}", ip),
            None => "Net: connected".into(),
        }
    } else {
        "Net: disconnected".into()
    };
    draw_small(d, &net_str, 0, 14);
    draw_small(d, if st.mqtt_connected { "MQTT: online" } else { "MQTT: offline" }, 0, 25);
    let uptime_s = unsafe { esp_idf_svc::sys::esp_timer_get_time() / 1_000_000 } as u64;
    let up_str = format!("Up: {}h {}m", uptime_s / 3600, (uptime_s % 3600) / 60);
    draw_small(d, &up_str, 0, 36);
    let reason = format!("Boot: {}", crate::watchdog::reboot_reason());
    draw_small(d, &reason, 0, 43);
}
