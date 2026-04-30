//! Shared application state accessible from HTTP handlers and main loop.

use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::cwl_data::CwlData;
use crate::scheduler::{ScheduleEntry, BypassSchedule};

/// Mutable application state shared between main loop and HTTP handlers.
#[derive(Debug)]
pub struct AppStateInner {
    pub cwl_data: CwlData,
    pub config: AppConfig,

    // Requested state (set by web/MQTT, consumed by OT polling)
    pub requested_vent_level: u8,
    pub requested_bypass_open: bool,
    pub requested_filter_reset: bool,

    // Schedule state
    pub schedule_active: bool,
    pub schedule_override: bool,
    pub schedules: Vec<ScheduleEntry>,
    pub bypass_schedule: BypassSchedule,

    // Timed off (read from scheduler)
    pub timed_off_active: bool,
    pub timed_off_remaining_min: u32,

    // Timed off NVS persistence
    pub timed_off_end_epoch: i64,

    // Network
    pub network_connected: bool,
    pub ip_address: Option<String>,
    pub mqtt_connected: bool,

    // Display framebuffer (128x64 pixels, 1 bit per pixel = 1024 bytes)
    pub display_framebuffer: [u8; 1024],

    // Flags
    pub simulated: bool,
    pub display_wake_requested: bool,
    pub encoder_action: Option<String>, // "left", "right", "press"
    pub timed_off_request: Option<u8>,  // hours to activate timed off
    pub cancel_timed_off: bool,
    pub persist_timed_off: bool, // flag to save timed-off state to NVS
}

impl AppStateInner {
    pub fn new(config: AppConfig) -> Self {
        let requested_vent_level = config.ventilation_level;
        let requested_bypass_open = config.bypass_open;
        Self {
            cwl_data: CwlData::new(),
            requested_vent_level,
            requested_bypass_open,
            requested_filter_reset: false,
            schedule_active: false,
            schedule_override: false,
            timed_off_active: false,
            timed_off_remaining_min: 0,
            timed_off_end_epoch: 0,
            schedules: Vec::new(),
            bypass_schedule: BypassSchedule::default(),
            network_connected: false,
            ip_address: None,
            mqtt_connected: false,
            display_framebuffer: [0u8; 1024],
            simulated: cfg!(feature = "simulate-ot"),
            display_wake_requested: false,
            encoder_action: None,
            timed_off_request: None,
            cancel_timed_off: false,
            persist_timed_off: false,
            config,
        }
    }
}

/// Thread-safe shared state handle.
pub type AppState = Arc<Mutex<AppStateInner>>;

pub fn new_app_state(config: AppConfig) -> AppState {
    Arc::new(Mutex::new(AppStateInner::new(config)))
}
