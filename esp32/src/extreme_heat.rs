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

use crate::app_state::{AppStateInner, EhEvent, Reason, EH_EVENT_CAPACITY};
use crate::cwl_data::VentLevel;
use crate::humidity;

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
    /// Set on the enable rising edge, cleared once we've captured the unit's
    /// settled level as our baseline (after the readiness gates). Defers baseline
    /// capture past the post-boot ID-77 mirror so that mirror isn't mistaken for a
    /// manual change that needlessly restarts the dwell.
    baseline_pending: bool,
}

impl ExtremeHeat {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            last_eval_ms: u32::MAX - EVAL_INTERVAL_MS, // first eval runs immediately
            was_enabled: false,
            baseline_pending: false,
        }
    }

    pub fn update(&mut self, now_ms: u32) {
        if now_ms.wrapping_sub(self.last_eval_ms) < EVAL_INTERVAL_MS {
            return;
        }
        self.last_eval_ms = now_ms;

        let mut st = self.state.lock().unwrap();
        let eh_on = st.config.extreme_heat_enabled;
        let prot_on = st.config.humidity_protection_enabled;

        // Extreme-heat mode not owning the level. Moisture protection may still
        // run on its own as a surgical raise-only override.
        if !eh_on {
            if self.was_enabled {
                self.was_enabled = false;
                st.mqtt_publish_extreme_heat = true;
            }
            if prot_on {
                Self::run_protection_only(&mut st, now_ms);
            } else if st.protection_active {
                // Protection just turned off via config — release our hold.
                st.protection_active = false;
                st.schedule_override = false;
                st.mqtt_publish_extreme_heat = true;
            }
            return;
        }

        // Extreme-heat owns the level. Rising edge: zero the dwell so the first
        // real decision applies immediately. The baseline level is captured below,
        // after the readiness gates, once requested_vent_level reflects the unit's
        // actual level — capturing it here (before the OT master mirrors ID 77)
        // would make that mirror look like a manual change and restart the dwell.
        if !self.was_enabled {
            self.was_enabled = true;
            self.baseline_pending = true;
            st.eh_last_change_epoch = 0;
            st.mqtt_publish_extreme_heat = true;
        }
        // While extreme-heat owns the level there is no separate protection hold.
        st.protection_active = false;

        if st.timed_off_active {
            return; // timed-off wins
        }
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        if now_epoch < EPOCH_VALID {
            return; // NTP not synced
        }
        if !st.cwl_data.connected {
            return; // need live temps (also for sensor fallbacks)
        }
        let supply = st.cwl_data.supply_inlet_temp;
        let exhaust = st.cwl_data.exhaust_inlet_temp;

        // Capture our baseline from the now-settled requested level (post-boot the
        // OT master has mirrored the unit's real level into it). The dwell timer
        // was zeroed on the rising edge, so the first genuine decision below still
        // applies promptly instead of being held for a full dwell.
        if self.baseline_pending {
            self.baseline_pending = false;
            st.eh_current_level = st.requested_vent_level;
        }

        // Manual override: requested level moved out from under us → adopt it and
        // wait a full dwell before overriding the manual choice.
        if st.requested_vent_level != st.eh_current_level {
            st.eh_current_level = st.requested_vent_level;
            st.eh_current_reason = Reason::Manual;
            st.eh_last_change_epoch = now_epoch;
            return;
        }

        let (level, reason) = decide(&st, now_ms, supply, exhaust, prot_on);
        // Never fully stop the fans: a fully-off CWL leaves stale air in the duct,
        // so the supply-inlet sensor no longer reads true outdoor temperature.
        // Floor the automatic decision at Reduced (manual Off is still honoured
        // via the separate manual-override path above).
        let level_u8 = (level as u8).max(VentLevel::Reduced as u8);
        if level_u8 == st.eh_current_level {
            st.eh_current_reason = reason; // keep level, refresh the reason for display
            return;
        }
        if now_epoch - st.eh_last_change_epoch < EH_DWELL_SECS {
            return; // dwell
        }
        record_change(&mut st, now_epoch, level_u8, reason);
        info!(
            "ExtremeHeat: level -> {} ({}) supply {:.1} / exhaust {:.1}",
            level_u8, reason.as_str(), supply, exhaust
        );
    }

    /// Standalone moisture protection (extreme-heat off): a raise-only override.
    /// While a protection condition holds it lifts the level and holds it via
    /// `schedule_override`; when the condition clears it releases so the schedule
    /// or manual level resumes.
    fn run_protection_only(st: &mut AppStateInner, now_ms: u32) {
        let now_epoch = unsafe { esp_idf_svc::sys::time(std::ptr::null_mut()) } as i64;
        if now_epoch < EPOCH_VALID || !st.cwl_data.connected {
            return;
        }
        let supply = st.cwl_data.supply_inlet_temp;
        let exhaust = st.cwl_data.exhaust_inlet_temp;
        let target = humidity::inputs(st, now_ms, exhaust, supply)
            .and_then(|inp| humidity::protection_level(&inp, st.protection_active));

        match target {
            Some(lvl) => {
                st.protection_active = true;
                st.schedule_override = true; // hold against the scheduler
                let want = (lvl as u8).max(st.requested_vent_level); // only raise
                let dwell_ok = st.eh_last_change_epoch == 0
                    || now_epoch - st.eh_last_change_epoch >= EH_DWELL_SECS;
                if want != st.eh_current_level && dwell_ok {
                    record_change(st, now_epoch, want, Reason::Dehumidify);
                    info!("Protection: raise to level {}", want);
                } else {
                    st.eh_current_reason = Reason::Dehumidify;
                }
            }
            None => {
                if st.protection_active {
                    // Release: hand control back to the schedule / manual level.
                    st.protection_active = false;
                    st.schedule_override = false;
                    st.eh_last_change_epoch = now_epoch;
                    st.eh_current_reason = Reason::TempDelta;
                    st.mqtt_publish_extreme_heat = true;
                    info!("Protection: released");
                }
            }
        }
    }
}

/// Compute the level + reason for extreme-heat mode. Humidity-aware (enthalpy +
/// moisture protection) when fresh sensor data is available, else temperature.
fn decide(
    st: &AppStateInner,
    now_ms: u32,
    supply: f32,
    exhaust: f32,
    prot_on: bool,
) -> (VentLevel, Reason) {
    match humidity::inputs(st, now_ms, exhaust, supply) {
        Some(inp) => {
            if prot_on {
                if let Some(lvl) = humidity::protection_level(&inp, st.protection_active) {
                    return (lvl, Reason::Dehumidify);
                }
            }
            let (lvl, cooling) = humidity::enthalpy_level(&inp);
            (lvl, if cooling { Reason::CoolingAssist } else { Reason::MuggySuppression })
        }
        None => (select_level(supply, exhaust), Reason::TempDelta),
    }
}

/// Apply a level change: update state, record a reason-tagged event, flag publish.
fn record_change(st: &mut AppStateInner, now_epoch: i64, level: u8, reason: Reason) {
    st.requested_vent_level = level;
    st.eh_current_level = level;
    st.eh_current_reason = reason;
    st.eh_last_change_epoch = now_epoch;
    st.initial_level_known = true;
    if st.eh_events.len() >= EH_EVENT_CAPACITY {
        st.eh_events.pop_front();
    }
    st.eh_events.push_back(EhEvent { epoch: now_epoch, level, reason });
    st.mqtt_publish_extreme_heat = true;
}

/// Restore the RAM-only parts of the activation state from a retained MQTT
/// snapshot. The enable flags are NOT restored (NVS config is authoritative).
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
    if let Some(r) = val["reason"].as_str() {
        st.eh_current_reason = Reason::from_str(r);
    }
    if let Some(events) = val["events"].as_array() {
        st.eh_events.clear();
        for e in events.iter().take(EH_EVENT_CAPACITY) {
            if let (Some(epoch), Some(level)) = (e["epoch"].as_i64(), e["level"].as_u64()) {
                let reason = Reason::from_str(e["reason"].as_str().unwrap_or("temp"));
                st.eh_events.push_back(EhEvent { epoch, level: level as u8, reason });
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
