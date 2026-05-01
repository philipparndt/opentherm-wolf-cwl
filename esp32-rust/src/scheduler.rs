//! Ventilation and bypass scheduling with manual override and timed-off support.

use std::sync::{Arc, Mutex};
use log::info;
use serde::{Deserialize, Serialize};

use crate::app_state::AppStateInner;

type AppState = Arc<Mutex<AppStateInner>>;

#[allow(dead_code)]
pub const MAX_SCHEDULES: usize = 16;
const EVAL_INTERVAL_MS: u32 = 60_000;

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
    pub active_days: u8, // bitmask: bit0=Mon..bit6=Sun
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

pub struct Scheduler {
    state: AppState,
    pub schedules: Vec<ScheduleEntry>,
    pub bypass_schedule: BypassSchedule,
    last_eval_ms: u32,
    last_active_index: i32,
    pub timed_off_active: bool,
    pub timed_off_end_epoch: i64,
}

impl Scheduler {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            schedules: Vec::new(),
            bypass_schedule: BypassSchedule::default(),
            last_eval_ms: 0,
            last_active_index: -1,
            timed_off_active: false,
            timed_off_end_epoch: 0,
        }
    }

    pub fn update(&mut self, now_ms: u32) {
        // Check timed off expiration
        if self.timed_off_active {
            let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
            if now_epoch > 1_700_000_000 && now_epoch >= self.timed_off_end_epoch {
                info!("Scheduler: Timed off expired");
                self.timed_off_active = false;
                self.timed_off_end_epoch = 0;
                let mut st = self.state.lock().unwrap();
                st.requested_vent_level = 2; // Normal
                st.schedule_override = false;
                st.timed_off_end_epoch = 0;
                st.persist_timed_off = true;
            }
        }

        // Update timed-off status in shared state
        {
            let mut st = self.state.lock().unwrap();
            st.timed_off_active = self.timed_off_active;
            st.timed_off_remaining_min = self.timed_off_remaining_minutes();
        }

        if now_ms.wrapping_sub(self.last_eval_ms) < EVAL_INTERVAL_MS {
            return;
        }
        self.last_eval_ms = now_ms;

        // Sync schedules from AppState (may have been updated via web UI)
        {
            let st = self.state.lock().unwrap();
            self.schedules = st.schedules.clone();
            self.bypass_schedule = st.bypass_schedule.clone();
        }

        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        if now_epoch < 1_700_000_000 {
            return; // NTP not synced yet
        }

        // Get local time
        let mut tm: esp_idf_svc::sys::tm = unsafe { std::mem::zeroed() };
        let time_t = now_epoch as esp_idf_svc::sys::time_t;
        unsafe { esp_idf_svc::sys::localtime_r(&time_t, &mut tm) };

        let hour = tm.tm_hour as u8;
        let minute = tm.tm_min as u8;
        let current_minutes = hour as u16 * 60 + minute as u16;
        // tm_wday: 0=Sun, convert to bitmask: bit0=Mon..bit6=Sun
        let day_bit: u8 = match tm.tm_wday {
            0 => 1 << 6, // Sunday
            d => 1 << (d - 1),
        };

        // Ventilation schedule
        if !self.timed_off_active {
            let match_index = self.evaluate_ventilation(current_minutes, day_bit);

            if match_index != self.last_active_index {
                self.last_active_index = match_index;
            }

            let mut st = self.state.lock().unwrap();
            if !st.schedule_override {
                if match_index >= 0 {
                    st.schedule_active = true;
                    let level = self.schedules[match_index as usize].vent_level;
                    if level != st.requested_vent_level {
                        st.requested_vent_level = level;
                        info!("Scheduler: Level set to {}", level);
                    }
                } else if st.schedule_active {
                    st.schedule_active = false;
                    st.requested_vent_level = st.config.ventilation_level;
                    info!("Scheduler: No schedule active, using default");
                }
            }
        }

        // Bypass schedule
        if self.bypass_schedule.enabled {
            let day = tm.tm_mday as u8;
            let month = (tm.tm_mon + 1) as u8;
            let is_summer = is_date_in_range(
                day, month,
                self.bypass_schedule.start_day, self.bypass_schedule.start_month,
                self.bypass_schedule.end_day, self.bypass_schedule.end_month,
            );
            let mut st = self.state.lock().unwrap();
            if is_summer != st.requested_bypass_open {
                st.requested_bypass_open = is_summer;
                info!("Scheduler: Bypass {}", if is_summer { "open (summer)" } else { "closed (winter)" });
            }
        }
    }

    fn evaluate_ventilation(&self, current_minutes: u16, day_bit: u8) -> i32 {
        for (i, entry) in self.schedules.iter().enumerate() {
            if !entry.enabled || (entry.active_days & day_bit) == 0 {
                continue;
            }
            let start = entry.start_hour as u16 * 60 + entry.start_minute as u16;
            let end = entry.end_hour as u16 * 60 + entry.end_minute as u16;
            if is_time_in_range(current_minutes, start, end) {
                return i as i32;
            }
        }
        -1
    }

    pub fn activate_timed_off(&mut self, hours: u8) {
        if hours == 0 || hours > 99 { return; }
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        if now_epoch < 1_700_000_000 { return; }

        self.timed_off_active = true;
        self.timed_off_end_epoch = now_epoch + (hours as i64) * 3600;

        let mut st = self.state.lock().unwrap();
        st.requested_vent_level = 0; // Off
        st.schedule_override = true;
        st.timed_off_end_epoch = self.timed_off_end_epoch;
        st.persist_timed_off = true;
        info!("Scheduler: Timed off for {}h", hours);
    }

    pub fn cancel_timed_off(&mut self) {
        self.timed_off_active = false;
        self.timed_off_end_epoch = 0;
        let mut st = self.state.lock().unwrap();
        st.requested_vent_level = 2; // Normal
        st.schedule_override = false;
        st.timed_off_end_epoch = 0;
        st.persist_timed_off = true;
        info!("Scheduler: Timed off cancelled");
    }

    pub fn timed_off_remaining_minutes(&self) -> u32 {
        if !self.timed_off_active { return 0; }
        let now = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        let remaining = self.timed_off_end_epoch - now;
        if remaining > 0 { (remaining / 60) as u32 } else { 0 }
    }
}

fn is_time_in_range(current: u16, start: u16, end: u16) -> bool {
    if start <= end {
        current >= start && current < end
    } else {
        // Crosses midnight
        current >= start || current < end
    }
}

fn is_date_in_range(day: u8, month: u8, start_day: u8, start_month: u8, end_day: u8, end_month: u8) -> bool {
    let current = (month as u16) * 100 + day as u16;
    let start = (start_month as u16) * 100 + start_day as u16;
    let end = (end_month as u16) * 100 + end_day as u16;
    if start <= end {
        current >= start && current <= end
    } else {
        current >= start || current <= end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_in_range_normal() {
        assert!(is_time_in_range(600, 480, 720)); // 10:00 in 8:00-12:00
        assert!(!is_time_in_range(300, 480, 720)); // 5:00 not in 8:00-12:00
    }

    #[test]
    fn test_time_in_range_midnight() {
        assert!(is_time_in_range(1400, 1320, 360)); // 23:20 in 22:00-06:00
        assert!(is_time_in_range(120, 1320, 360));   // 02:00 in 22:00-06:00
        assert!(!is_time_in_range(600, 1320, 360));  // 10:00 not in 22:00-06:00
    }

    #[test]
    fn test_date_in_range_normal() {
        assert!(is_date_in_range(15, 6, 15, 4, 30, 9)); // Jun 15 in Apr 15 - Sep 30
        assert!(!is_date_in_range(1, 2, 15, 4, 30, 9)); // Feb 1 not in Apr 15 - Sep 30
    }

    #[test]
    fn test_date_in_range_year_wrap() {
        assert!(is_date_in_range(15, 11, 1, 10, 31, 3)); // Nov 15 in Oct 1 - Mar 31
        assert!(is_date_in_range(15, 2, 1, 10, 31, 3));   // Feb 15 in Oct 1 - Mar 31
        assert!(!is_date_in_range(15, 6, 1, 10, 31, 3));  // Jun 15 not in Oct 1 - Mar 31
    }
}
