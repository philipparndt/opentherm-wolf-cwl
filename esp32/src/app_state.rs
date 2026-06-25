//! Shared application state accessible from HTTP handlers and main loop.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::cwl_data::CwlData;
use crate::history::TempHistory;
use crate::scheduler::{ScheduleEntry, BypassSchedule};

/// Maximum number of extreme-heat level-change events retained for the web
/// graph markers. Covers more than the ~24 h history window even if the mode
/// changes level on every dwell boundary; oldest dropped when full.
pub const EH_EVENT_CAPACITY: usize = 64;

/// Why the ventilation mode settled on its current level — surfaced in the UI so
/// the (multi-factor) decision is legible, and attached to each change event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reason {
    #[default]
    TempDelta,        // supply-vs-exhaust temperature bands (fallback)
    CoolingAssist,    // outdoor air is lower-energy (enthalpy) → ventilate to cool
    Dehumidify,       // moisture protection: indoor too humid, outdoor drier
    MuggySuppression, // outdoor air is higher-energy (humid) → hold ventilation down
    Manual,           // user set the level (web / encoder / MQTT)
    Schedule,         // a ventilation schedule changed the level
    Reboot,           // device (re)started — a marker only, not a level decision
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::TempDelta => "temp",
            Reason::CoolingAssist => "cooling",
            Reason::Dehumidify => "dehumidify",
            Reason::MuggySuppression => "muggy",
            Reason::Manual => "manual",
            Reason::Schedule => "schedule",
            Reason::Reboot => "reboot",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "cooling" => Reason::CoolingAssist,
            "dehumidify" => Reason::Dehumidify,
            "muggy" => Reason::MuggySuppression,
            "manual" => Reason::Manual,
            "schedule" => Reason::Schedule,
            "reboot" => Reason::Reboot,
            _ => Reason::TempDelta,
        }
    }
}

/// A ventilation level change made by the mode, kept so the web UI can draw a
/// reason-annotated marker at the time it happened.
#[derive(Debug, Clone, Copy)]
pub struct EhEvent {
    pub epoch: i64,
    pub level: u8,
    pub reason: Reason,
}

/// Latest reading from one MQTT humidity sensor. `temperature`/`pressure` may be
/// absent depending on the sensor; `humidity` is always present.
#[derive(Debug, Clone, Copy)]
pub struct HumiditySample {
    pub humidity: f32,
    pub temperature: Option<f32>,
    pub pressure: Option<f32>,
    pub updated_ms: u32,
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
    pub eh_current_reason: Reason,

    // Humidity-aware ventilation runtime state. Sensor readings come from MQTT
    // (keyed by topic for indoor; one slot for outdoor). `ambient_pressure_kpa`
    // is updated from any sensor reporting pressure. `protection_active` tracks
    // the standalone moisture-protection override (with hysteresis).
    pub humidity_inside: HashMap<String, HumiditySample>,
    pub humidity_outside: Option<HumiditySample>,
    pub ambient_pressure_kpa: f32,
    pub protection_active: bool,

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
            eh_current_reason: Reason::TempDelta,
            humidity_inside: HashMap::new(),
            humidity_outside: None,
            ambient_pressure_kpa: crate::psychro::STANDARD_PRESSURE_KPA,
            protection_active: false,
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

    /// Record a history marker (a level change or a reboot) for the web chart,
    /// capped at [`EH_EVENT_CAPACITY`] (oldest dropped), and flag the retained
    /// snapshot for republish so markers survive a reboot. De-duplicates an
    /// identical (level, reason) at the same second so repeated calls from
    /// different code paths don't stack markers.
    pub fn push_history_marker(&mut self, epoch: i64, level: u8, reason: Reason) {
        if let Some(last) = self.eh_events.back() {
            if last.epoch == epoch && last.level == level && last.reason == reason {
                return;
            }
        }
        if self.eh_events.len() >= EH_EVENT_CAPACITY {
            self.eh_events.pop_front();
        }
        self.eh_events.push_back(EhEvent { epoch, level, reason });
        self.mqtt_publish_extreme_heat = true;
    }

    /// Like [`push_history_marker`] but stamps the current wall-clock time.
    /// No-op until the clock is NTP-synced (a marker needs a real timestamp to
    /// place it on the chart).
    pub fn push_history_marker_now(&mut self, level: u8, reason: Reason) {
        let epoch = unsafe { esp_idf_svc::sys::time(core::ptr::null_mut()) } as i64;
        if epoch < 1_700_000_000 {
            return;
        }
        self.push_history_marker(epoch, level, reason);
    }
}

/// Thread-safe shared state handle.
pub type AppState = Arc<Mutex<AppStateInner>>;

pub fn new_app_state(config: AppConfig) -> AppState {
    Arc::new(Mutex::new(AppStateInner::new(config)))
}
