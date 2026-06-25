//! Extreme-heat automatic ventilation control.
//!
//! When `config.extreme_heat_enabled` is set, this state machine drives
//! `requested_vent_level` from the difference between the supply inlet air
//! (OpenTherm ID 80) and the exhaust inlet air (ID 82):
//!
//! ```text
//!   delta = supply - exhaust
//!   delta > +0.5 °C            -> Off      (incoming air much hotter than indoor)
//!   0 °C < delta <= +0.5 °C    -> Reduced  (slightly hotter)
//!   -1.0 °C <= delta < 0 °C    -> Normal   (cooler)
//!   delta < -1.0 °C            -> Party    (much cooler — pull in the cool air)
//! ```
//!
//! A chosen level is held for at least [`EH_DWELL_SECS`] before the mode may
//! change it again, so readings hovering near a band boundary don't toggle the
//! fans. Manual changes and timed-off take precedence; see [`ExtremeHeat::update`].

use std::sync::{Arc, Mutex};

use log::info;

use crate::app_state::{AppStateInner, EhEvent, EH_EVENT_CAPACITY};
use crate::cwl_data::VentLevel;

type AppState = Arc<Mutex<AppStateInner>>;

/// Re-evaluate at most once per minute — far finer than the dwell, identical
/// cadence to the scheduler.
const EVAL_INTERVAL_MS: u32 = 60_000;

/// Above this delta (supply hotter than exhaust by more than this) → Off.
pub const EH_OFF_DELTA: f32 = 0.5;
/// Below this delta (supply cooler than exhaust by more than this) → Party.
pub const EH_PARTY_DELTA: f32 = -1.0;
/// Minimum time a mode-driven decision is held before it can change again.
pub const EH_DWELL_SECS: i64 = 15 * 60;

/// Epoch threshold below which the wall clock is considered unsynced (NTP not
/// yet acquired). Mirrors the guard used by the scheduler.
const EPOCH_VALID: i64 = 1_700_000_000;

/// Map the supply/exhaust temperature difference to a ventilation level.
pub fn select_level(supply: f32, exhaust: f32) -> VentLevel {
    let delta = supply - exhaust;
    if delta > EH_OFF_DELTA {
        VentLevel::Off
    } else if delta > 0.0 {
        VentLevel::Reduced
    } else if delta >= EH_PARTY_DELTA {
        VentLevel::Normal
    } else {
        VentLevel::Party
    }
}

pub struct ExtremeHeat {
    state: AppState,
    last_eval_ms: u32,
    /// Previous value of `config.extreme_heat_enabled`, used to detect the
    /// disabled→enabled edge so we adopt the current level and apply the first
    /// decision immediately rather than after a full dwell.
    was_enabled: bool,
}

impl ExtremeHeat {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            last_eval_ms: u32::MAX - EVAL_INTERVAL_MS, // first eval runs immediately
            was_enabled: false,
        }
    }

    pub fn update(&mut self, now_ms: u32) {
        if now_ms.wrapping_sub(self.last_eval_ms) < EVAL_INTERVAL_MS {
            return;
        }
        self.last_eval_ms = now_ms;

        let mut st = self.state.lock().unwrap();

        // Disabled: stay fully inert. Track the edge so re-enabling adopts the
        // level the user/schedule left in place.
        if !st.config.extreme_heat_enabled {
            if self.was_enabled {
                // Falling edge — publish the disabled state for persistence.
                st.mqtt_publish_extreme_heat = true;
            }
            self.was_enabled = false;
            return;
        }

        // Rising edge: adopt whatever level is currently requested and allow the
        // first decision to apply right away (epoch 0 is always past the dwell).
        if !self.was_enabled {
            self.was_enabled = true;
            st.eh_current_level = st.requested_vent_level;
            st.eh_last_change_epoch = 0;
            st.mqtt_publish_extreme_heat = true;
        }

        // Timed-off wins: don't raise the level while an off-timer is running.
        if st.timed_off_active {
            return;
        }

        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        if now_epoch < EPOCH_VALID {
            return; // NTP not synced — leave the level untouched
        }

        // Need live, valid temperatures to decide.
        if !st.cwl_data.connected {
            return;
        }
        let supply = st.cwl_data.supply_inlet_temp;
        let exhaust = st.cwl_data.exhaust_inlet_temp;

        // Manual override: if the requested level no longer matches what the mode
        // last settled on, the user (web / encoder / MQTT) moved it. Adopt that
        // as the current decision and restart the dwell so the mode waits a full
        // window before overriding the manual choice.
        if st.requested_vent_level != st.eh_current_level {
            st.eh_current_level = st.requested_vent_level;
            st.eh_last_change_epoch = now_epoch;
            return;
        }

        let selected = select_level(supply, exhaust) as u8;
        if selected == st.eh_current_level {
            return; // already there — nothing to do, dwell timer untouched
        }
        if now_epoch - st.eh_last_change_epoch < EH_DWELL_SECS {
            return; // within the dwell window — hold the current level
        }

        // Apply the new decision.
        st.requested_vent_level = selected;
        st.eh_current_level = selected;
        st.eh_last_change_epoch = now_epoch;
        st.initial_level_known = true;
        if st.eh_events.len() >= EH_EVENT_CAPACITY {
            st.eh_events.pop_front();
        }
        st.eh_events.push_back(EhEvent { epoch: now_epoch, level: selected });
        st.mqtt_publish_extreme_heat = true;
        info!(
            "ExtremeHeat: level -> {} (supply {:.1} / exhaust {:.1}, delta {:.1})",
            selected, supply, exhaust, supply - exhaust
        );
    }
}

/// Restore the RAM-only parts of the extreme-heat activation state from a
/// retained MQTT snapshot (see `MqttManager::publish_extreme_heat_snapshot`).
/// The `enabled` flag is intentionally NOT restored here — NVS config stays the
/// single authority for whether the mode is on. Malformed input is a no-op.
pub fn restore_snapshot(st: &mut AppStateInner, json: &[u8]) {
    let val: serde_json::Value = match serde_json::from_slice(json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(level) = val["currentLevel"].as_u64() {
        st.eh_current_level = level as u8;
    }
    if let Some(epoch) = val["lastChangeEpoch"].as_i64() {
        st.eh_last_change_epoch = epoch;
    }
    if let Some(events) = val["events"].as_array() {
        st.eh_events.clear();
        for e in events.iter().take(EH_EVENT_CAPACITY) {
            if let (Some(epoch), Some(level)) = (e["epoch"].as_i64(), e["level"].as_u64()) {
                st.eh_events.push_back(EhEvent { epoch, level: level as u8 });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::select_level;
    use crate::cwl_data::VentLevel;

    #[test]
    fn band_off_when_supply_much_hotter() {
        // delta = +1.0 > 0.5
        assert_eq!(select_level(23.0, 22.0), VentLevel::Off);
    }

    #[test]
    fn band_reduced_when_slightly_hotter() {
        // delta = +0.3, in (0, 0.5]
        assert_eq!(select_level(22.3, 22.0), VentLevel::Reduced);
    }

    #[test]
    fn band_normal_when_cooler() {
        // delta = -0.5, in [-1.0, 0)
        assert_eq!(select_level(21.5, 22.0), VentLevel::Normal);
    }

    #[test]
    fn band_party_when_much_cooler() {
        // delta = -2.0 < -1.0
        assert_eq!(select_level(20.0, 22.0), VentLevel::Party);
    }

    #[test]
    fn boundary_delta_zero_is_normal() {
        // supply not strictly greater than exhaust → Normal
        assert_eq!(select_level(22.0, 22.0), VentLevel::Normal);
    }

    #[test]
    fn boundary_delta_plus_half_is_reduced() {
        // exactly +0.5 is not > 0.5 → Reduced
        assert_eq!(select_level(22.5, 22.0), VentLevel::Reduced);
    }

    #[test]
    fn boundary_delta_minus_one_is_normal() {
        // exactly -1.0 is >= -1.0 → Normal
        assert_eq!(select_level(21.0, 22.0), VentLevel::Normal);
    }

    #[test]
    fn boundary_just_past_minus_one_is_party() {
        assert_eq!(select_level(20.99, 22.0), VentLevel::Party);
    }
}
