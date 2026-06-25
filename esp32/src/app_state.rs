//! Shared application state accessible from HTTP handlers and main loop.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::cwl_data::CwlData;
use crate::history::TempHistory;
use crate::scheduler::{ScheduleEntry, BypassSchedule};

/// Maximum number of extreme-heat level-change events retained for the web
/// graph markers. Covers more than the ~24 h history window even if the mode
/// changes level on every dwell boundary; oldest dropped when full.
pub const EH_EVENT_CAPACITY: usize = 64;

/// A ventilation level change made by extreme-heat mode, kept so the web UI
/// can draw a marker at the time it happened.
#[derive(Debug, Clone, Copy)]
pub struct EhEvent {
    pub epoch: i64,
    pub level: u8,
}

/// Mutable application state shared between main loop and HTTP handlers.
#[derive(Debug)]
pub struct AppStateInner {
    pub cwl_data: CwlData,
    pub temp_history: TempHistory,
    pub config: AppConfig,

    // Requested state (set by web/MQTT, consumed by OT polling)
    pub requested_vent_level: u8,
    pub requested_bypass_open: bool,
    pub requested_filter_reset: bool,
    // True once the OT master has either (a) read the CWL's current ventilation
    // level via ID 77 and mirrored it into requested_vent_level, or (b) the
    // user explicitly set a level via display / MQTT / web. Until then the OT
    // master skips the Setpoint (ID 71) WRITE so it doesn't override the unit
    // with whatever stale value happened to be in config on boot.
    pub initial_level_known: bool,

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

    // Extreme-heat mode runtime state (config.extreme_heat_enabled is the
    // on/off switch). eh_current_level is the level the mode last settled on;
    // eh_last_change_epoch gates the 15-minute decision dwell; eh_events feeds
    // the web graph markers.
    pub eh_current_level: u8,
    pub eh_last_change_epoch: i64,
    pub eh_events: VecDeque<EhEvent>,

    // MQTT-backed RAM-only persistence (no flash). The MQTT receive callback
    // drops raw retained snapshots here for the main loop to parse & apply
    // (heavy JSON work kept off the callback thread). The publish flags are
    // raised by the producers (history rollover / extreme-heat change) and
    // consumed by the MQTT thread.
    pub pending_history_json: Option<Vec<u8>>,
    pub pending_extreme_heat_json: Option<Vec<u8>>,
    pub mqtt_publish_history: bool,
    pub mqtt_publish_extreme_heat: bool,
    // One-shot recovery guards: only the first retained snapshot per topic after
    // boot is applied; later echoes (incl. our own publishes) are ignored.
    pub history_recovered: bool,
    pub extreme_heat_recovered: bool,

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
    pub timed_off_request: Option<u16>, // minutes to activate timed off
    pub cancel_timed_off: bool,
    pub persist_timed_off: bool, // flag to save timed-off state to NVS
    pub persist_config: bool,    // flag to save full config to NVS
    pub persist_schedules: bool, // flag to save ventilation + bypass schedules to NVS
}

impl AppStateInner {
    pub fn new(config: AppConfig) -> Self {
        let requested_vent_level = config.ventilation_level;
        let requested_bypass_open = config.bypass_open;
        Self {
            cwl_data: CwlData::new(),
            temp_history: TempHistory::new(),
            requested_vent_level,
            requested_bypass_open,
            requested_filter_reset: false,
            initial_level_known: false,
            schedule_active: false,
            schedule_override: false,
            timed_off_active: false,
            timed_off_remaining_min: 0,
            timed_off_end_epoch: 0,
            eh_current_level: requested_vent_level,
            eh_last_change_epoch: 0,
            eh_events: VecDeque::with_capacity(EH_EVENT_CAPACITY),
            pending_history_json: None,
            pending_extreme_heat_json: None,
            mqtt_publish_history: false,
            mqtt_publish_extreme_heat: false,
            history_recovered: false,
            extreme_heat_recovered: false,
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
            persist_config: false,
            persist_schedules: false,
            config,
        }
    }
}

/// Thread-safe shared state handle.
pub type AppState = Arc<Mutex<AppStateInner>>;

pub fn new_app_state(config: AppConfig) -> AppState {
    Arc::new(Mutex::new(AppStateInner::new(config)))
}
