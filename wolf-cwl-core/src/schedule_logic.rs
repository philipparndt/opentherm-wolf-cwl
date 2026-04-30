//! Schedule evaluation logic — pure functions, no ESP-IDF dependency.

use serde::{Deserialize, Serialize};

pub const MAX_SCHEDULES: usize = 16;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub enabled: bool,
    #[serde(rename = "startHour")]
    pub start_hour: u8,
    #[serde(rename = "startMinute")]
    pub start_minute: u8,
    #[serde(rename = "endHour")]
    pub end_hour: u8,
    #[serde(rename = "endMinute")]
    pub end_minute: u8,
    #[serde(rename = "ventLevel")]
    pub vent_level: u8,
    #[serde(rename = "activeDays")]
    pub active_days: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BypassSchedule {
    pub enabled: bool,
    #[serde(rename = "startDay")]
    pub start_day: u8,
    #[serde(rename = "startMonth")]
    pub start_month: u8,
    #[serde(rename = "endDay")]
    pub end_day: u8,
    #[serde(rename = "endMonth")]
    pub end_month: u8,
}

pub fn is_time_in_range(current: u16, start: u16, end: u16) -> bool {
    if start <= end { current >= start && current < end }
    else { current >= start || current < end }
}

pub fn is_date_in_range(day: u8, month: u8, sd: u8, sm: u8, ed: u8, em: u8) -> bool {
    let c = (month as u16) * 100 + day as u16;
    let s = (sm as u16) * 100 + sd as u16;
    let e = (em as u16) * 100 + ed as u16;
    if s <= e { c >= s && c <= e } else { c >= s || c <= e }
}

pub fn wday_to_bitmask(wday: u8) -> u8 {
    match wday { 0 => 1 << 6, d => 1 << (d - 1) }
}

pub fn evaluate_ventilation(schedules: &[ScheduleEntry], current_minutes: u16, day_bit: u8) -> i32 {
    for (i, e) in schedules.iter().enumerate() {
        if !e.enabled || (e.active_days & day_bit) == 0 { continue; }
        let s = e.start_hour as u16 * 60 + e.start_minute as u16;
        let end = e.end_hour as u16 * 60 + e.end_minute as u16;
        if is_time_in_range(current_minutes, s, end) { return i as i32; }
    }
    -1
}

pub fn evaluate_bypass(schedule: &BypassSchedule, day: u8, month: u8) -> bool {
    schedule.enabled && is_date_in_range(day, month, schedule.start_day, schedule.start_month, schedule.end_day, schedule.end_month)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_normal() {
        assert!(is_time_in_range(600, 480, 720));
        assert!(is_time_in_range(480, 480, 720));
        assert!(!is_time_in_range(720, 480, 720));
        assert!(!is_time_in_range(300, 480, 720));
    }

    #[test]
    fn time_midnight_wrap() {
        assert!(is_time_in_range(1400, 1320, 360));
        assert!(is_time_in_range(120, 1320, 360));
        assert!(!is_time_in_range(600, 1320, 360));
    }

    #[test]
    fn date_normal() {
        assert!(is_date_in_range(15, 6, 15, 4, 30, 9));
        assert!(!is_date_in_range(1, 2, 15, 4, 30, 9));
    }

    #[test]
    fn date_year_wrap() {
        assert!(is_date_in_range(15, 11, 1, 10, 31, 3));
        assert!(is_date_in_range(15, 2, 1, 10, 31, 3));
        assert!(!is_date_in_range(15, 6, 1, 10, 31, 3));
    }

    #[test]
    fn wday() {
        assert_eq!(wday_to_bitmask(0), 0b01000000);
        assert_eq!(wday_to_bitmask(1), 0b00000001);
        assert_eq!(wday_to_bitmask(5), 0b00010000);
    }

    #[test]
    fn eval_no_schedules() {
        assert_eq!(evaluate_ventilation(&[], 600, 0x01), -1);
    }

    #[test]
    fn eval_matching() {
        let s = vec![ScheduleEntry { enabled: true, start_hour: 8, start_minute: 0, end_hour: 22, end_minute: 0, vent_level: 2, active_days: 0x7F }];
        assert_eq!(evaluate_ventilation(&s, 600, 0x01), 0);
        assert_eq!(evaluate_ventilation(&s, 300, 0x01), -1);
    }

    #[test]
    fn eval_disabled() {
        let s = vec![ScheduleEntry { enabled: false, start_hour: 0, start_minute: 0, end_hour: 23, end_minute: 59, vent_level: 2, active_days: 0x7F }];
        assert_eq!(evaluate_ventilation(&s, 600, 0x01), -1);
    }

    #[test]
    fn eval_wrong_day() {
        let s = vec![ScheduleEntry { enabled: true, start_hour: 8, start_minute: 0, end_hour: 22, end_minute: 0, vent_level: 2, active_days: 0x1F }];
        assert_eq!(evaluate_ventilation(&s, 600, 0x20), -1);
        assert_eq!(evaluate_ventilation(&s, 600, 0x01), 0);
    }

    #[test]
    fn eval_multiple() {
        let s = vec![
            ScheduleEntry { enabled: true, start_hour: 0, start_minute: 0, end_hour: 8, end_minute: 0, vent_level: 1, active_days: 0x7F },
            ScheduleEntry { enabled: true, start_hour: 8, start_minute: 0, end_hour: 13, end_minute: 0, vent_level: 2, active_days: 0x7F },
            ScheduleEntry { enabled: true, start_hour: 15, start_minute: 0, end_hour: 18, end_minute: 0, vent_level: 2, active_days: 0x7F },
        ];
        assert_eq!(evaluate_ventilation(&s, 120, 0x01), 0);
        assert_eq!(evaluate_ventilation(&s, 600, 0x01), 1);
        assert_eq!(evaluate_ventilation(&s, 840, 0x01), -1);
        assert_eq!(evaluate_ventilation(&s, 960, 0x01), 2);
    }

    #[test]
    fn bypass_disabled() {
        let b = BypassSchedule { enabled: false, start_day: 15, start_month: 4, end_day: 30, end_month: 9 };
        assert!(!evaluate_bypass(&b, 15, 6));
    }

    #[test]
    fn bypass_summer() {
        let b = BypassSchedule { enabled: true, start_day: 15, start_month: 4, end_day: 30, end_month: 9 };
        assert!(evaluate_bypass(&b, 15, 6));
        assert!(!evaluate_bypass(&b, 15, 1));
    }

    #[test]
    fn json_roundtrip() {
        let e = ScheduleEntry { enabled: true, start_hour: 8, start_minute: 30, end_hour: 22, end_minute: 0, vent_level: 2, active_days: 0x1F };
        let json = serde_json::to_string(&e).unwrap();
        let p: ScheduleEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(p.start_hour, 8);
        assert_eq!(p.vent_level, 2);
    }
}
