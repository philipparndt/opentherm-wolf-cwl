//! Internationalization — English and German string tables for the OLED display.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    En = 0,
    De = 1,
}

impl Language {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::De,
            _ => Self::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    pub fn from_code(s: &str) -> Self {
        match s {
            "de" => Self::De,
            _ => Self::En,
        }
    }
}

pub struct Strings {
    // Page headers
    pub ventilation: &'static str,
    pub set_level: &'static str,
    pub off_duration: &'static str,
    pub summer_mode: &'static str,
    pub set_mode: &'static str,
    pub intake: &'static str,
    pub outlet: &'static str,
    pub status: &'static str,
    pub system: &'static str,
    pub settings: &'static str,

    // Ventilation levels
    pub level_off: &'static str,
    pub level_reduced: &'static str,
    pub level_normal: &'static str,
    pub level_party: &'static str,
    pub level_schedule: &'static str,

    // Bypass / mode
    pub summer: &'static str,
    pub winter: &'static str,
    pub active: &'static str,
    pub inactive: &'static str,
    pub bypass_open_desc: &'static str,
    pub heat_recovery_desc: &'static str,

    // Status labels
    pub fault: &'static str,
    pub filter: &'static str,
    pub mode: &'static str,
    pub airflow: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
    pub replace: &'static str,
    pub ok: &'static str,

    // System labels
    pub net_connected: &'static str,
    pub net_disconnected: &'static str,
    pub mqtt_online: &'static str,
    pub mqtt_offline: &'static str,
    pub uptime_prefix: &'static str,
    pub boot_prefix: &'static str,

    // Edit hints
    pub hint_rotate_adjust: &'static str,
    pub hint_rotate_hours: &'static str,
    pub hint_rotate_toggle: &'static str,

    // Overlays
    pub connected: &'static str,
    pub disconnected: &'static str,
    pub no_network: &'static str,
    pub cwl_disconnected: &'static str,

    // Boot
    pub connecting: &'static str,

    // Temp labels
    pub supply: &'static str,
    pub exhaust: &'static str,

    // Settings page
    pub language_label: &'static str,
    pub english: &'static str,
    pub deutsch: &'static str,

    // Timed off
    pub resumes_in: &'static str,
}

pub const EN: Strings = Strings {
    ventilation: "Ventilation",
    set_level: "Set Level",
    off_duration: "Off Duration",
    summer_mode: "Summer Mode",
    set_mode: "Set Mode",
    intake: "Intake",
    outlet: "Outlet",
    status: "Status",
    system: "System",
    settings: "Settings",

    level_off: "Off",
    level_reduced: "Reduced",
    level_normal: "Normal",
    level_party: "Party",
    level_schedule: "Schedule",

    summer: "Summer",
    winter: "Winter",
    active: "Active",
    inactive: "Inactive",
    bypass_open_desc: "Bypass open - free cooling",
    heat_recovery_desc: "Heat recovery active",

    fault: "Fault:",
    filter: "Filter:",
    mode: "Mode:",
    airflow: "Airflow:",
    yes: "YES",
    no: "No",
    replace: "REPLACE",
    ok: "OK",

    net_connected: "Net: connected",
    net_disconnected: "Net: disconnected",
    mqtt_online: "MQTT: online",
    mqtt_offline: "MQTT: offline",
    uptime_prefix: "Up:",
    boot_prefix: "Boot:",

    hint_rotate_adjust: "rotate: adjust  press: ok",
    hint_rotate_hours: "rotate: hours  press: ok",
    hint_rotate_toggle: "rotate: toggle  press: ok",

    connected: "Connected",
    disconnected: "Disconnected",
    no_network: "No network",
    cwl_disconnected: "Disconnected",

    connecting: "Connecting...",

    supply: "Supply:",
    exhaust: "Exhaust:",

    language_label: "Language",
    english: "English",
    deutsch: "Deutsch",

    resumes_in: "Resumes in",
};

pub const DE: Strings = Strings {
    ventilation: "L\u{fc}ftung",
    set_level: "Stufe",
    off_duration: "Aus-Dauer",
    summer_mode: "Sommermodus",
    set_mode: "Modus",
    intake: "Zuluft",
    outlet: "Abluft",
    status: "Status",
    system: "System",
    settings: "Einstellungen",

    level_off: "Aus",
    level_reduced: "Reduziert",
    level_normal: "Normal",
    level_party: "Party",
    level_schedule: "Zeitplan",

    summer: "Sommer",
    winter: "Winter",
    active: "Aktiv",
    inactive: "Inaktiv",
    bypass_open_desc: "Bypass offen - K\u{fc}hlung",
    heat_recovery_desc: "W\u{e4}rmer\u{fc}ckgewinnung",

    fault: "St\u{f6}rung:",
    filter: "Filter:",
    mode: "Modus:",
    airflow: "Luftstrom:",
    yes: "JA",
    no: "Nein",
    replace: "WECHSELN",
    ok: "OK",

    net_connected: "Netz: verbunden",
    net_disconnected: "Netz: getrennt",
    mqtt_online: "MQTT: online",
    mqtt_offline: "MQTT: offline",
    uptime_prefix: "An:",
    boot_prefix: "Start:",

    hint_rotate_adjust: "drehen: w\u{e4}hlen  dr\u{fc}cken: ok",
    hint_rotate_hours: "drehen: Stunden  dr\u{fc}cken: ok",
    hint_rotate_toggle: "drehen: umschalten  dr\u{fc}cken: ok",

    connected: "Verbunden",
    disconnected: "Getrennt",
    no_network: "Kein Netzwerk",
    cwl_disconnected: "Getrennt",

    connecting: "Verbinde...",

    supply: "Zuluft:",
    exhaust: "Abluft:",

    language_label: "Sprache",
    english: "English",
    deutsch: "Deutsch",

    resumes_in: "Weiter in",
};

pub fn tr(lang: Language) -> &'static Strings {
    match lang {
        Language::En => &EN,
        Language::De => &DE,
    }
}

/// Get level name in the active language
pub fn level_name(lang: Language, level: u8) -> &'static str {
    let s = tr(lang);
    match level {
        0 => s.level_off,
        1 => s.level_reduced,
        2 => s.level_normal,
        3 => s.level_party,
        4 => s.level_schedule,
        _ => s.level_normal,
    }
}
