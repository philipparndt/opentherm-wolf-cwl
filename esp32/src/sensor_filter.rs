//! Guard against implausible MQTT sensor readings.
//!
//! Two layers, applied per sensor and per field as readings arrive:
//!
//! 1. **Hard physical bounds** — a value outside the physically-possible range is
//!    dropped outright and never influences the filter, so a stuck error code
//!    (e.g. a DHT bad read of `0 %` RH, which is never a real ambient value)
//!    cannot poison the history or "confirm" itself.
//! 2. **Spike guard** — a value that jumps further than `max_step` from the last
//!    trusted value "in no time" is held back as a suspected glitch. It is only
//!    accepted if the *next* reading confirms it (lands within `max_step` of this
//!    one), so a genuine step change still gets through after one sample. A run
//!    of `MAX_CONSECUTIVE_REJECTS` forces a resync so a real sustained shift (or
//!    a sensor recalibration) can never lock the channel out permanently.
//!
//! While a reading is held, the last accepted value is reused, smoothing over
//! isolated spikes instead of letting them drive the climate decision.

/// Largest change between consecutive readings accepted without confirmation.
pub const HUMIDITY_MAX_STEP: f32 = 10.0; // % RH
pub const TEMPERATURE_MAX_STEP: f32 = 8.0; // °C

/// Hard physical bounds. RH is floored at 1 %: 0 % is never a real ambient
/// reading (the classic DHT failure mode), so it is dropped, not merely smoothed.
pub const HUMIDITY_MIN: f32 = 1.0;
pub const HUMIDITY_MAX: f32 = 100.0;
pub const TEMPERATURE_MIN: f32 = -60.0;
pub const TEMPERATURE_MAX: f32 = 90.0;

/// Accept the next in-range reading after this many consecutive rejects so a
/// real sustained shift resyncs instead of locking the channel out.
pub const MAX_CONSECUTIVE_REJECTS: u8 = 3;

/// Whether `v` is finite and within `[lo, hi]`.
pub fn in_range(v: f32, lo: f32, hi: f32) -> bool {
    v.is_finite() && v >= lo && v <= hi
}

/// Spike-filter state for one field of one sensor, carried across readings.
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldFilter {
    accepted: Option<f32>,
    last_raw: Option<f32>,
    rejects: u8,
}

impl FieldFilter {
    /// Feed a raw reading; returns the trusted value to use (the last accepted
    /// value when this reading is held back), or `None` if nothing is trusted
    /// yet. `lo`/`hi` are the hard physical bounds; `max_step` the spike limit.
    pub fn update(&mut self, raw: f32, max_step: f32, lo: f32, hi: f32) -> Option<f32> {
        // Hard physical bound — drop without touching state, so repeated garbage
        // can neither be accepted nor "confirm" a later equally-bad reading.
        if !in_range(raw, lo, hi) {
            return self.accepted;
        }
        let accept = match (self.accepted, self.last_raw) {
            (None, _) => true,                                          // first value
            (Some(acc), _) if (raw - acc).abs() <= max_step => true,    // normal change
            (_, Some(prev)) if (raw - prev).abs() <= max_step => true,  // confirmed step
            _ => self.rejects >= MAX_CONSECUTIVE_REJECTS,               // resync escape
        };
        self.last_raw = Some(raw);
        if accept {
            self.accepted = Some(raw);
            self.rejects = 0;
        } else {
            self.rejects = self.rejects.saturating_add(1);
        }
        self.accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = HUMIDITY_MAX_STEP;
    const LO: f32 = HUMIDITY_MIN;
    const HI: f32 = HUMIDITY_MAX;

    fn upd(f: &mut FieldFilter, v: f32) -> Option<f32> {
        f.update(v, STEP, LO, HI)
    }

    #[test]
    fn first_reading_is_accepted() {
        let mut f = FieldFilter::default();
        assert_eq!(upd(&mut f, 45.0), Some(45.0));
    }

    #[test]
    fn small_change_is_accepted() {
        let mut f = FieldFilter::default();
        upd(&mut f, 45.0);
        assert_eq!(upd(&mut f, 50.0), Some(50.0));
    }

    #[test]
    fn isolated_spike_is_rejected_and_holds_last_good() {
        let mut f = FieldFilter::default();
        upd(&mut f, 20.0);
        // A glitch jump to 80 % (Δ60) with no confirmation → held at 20 %.
        assert_eq!(upd(&mut f, 80.0), Some(20.0));
        // Sensor returns to normal → tracked again.
        assert_eq!(upd(&mut f, 21.0), Some(21.0));
    }

    #[test]
    fn confirmed_step_change_is_accepted_on_second_reading() {
        let mut f = FieldFilter::default();
        upd(&mut f, 20.0);
        // Big jump → held the first time ...
        assert_eq!(upd(&mut f, 60.0), Some(20.0));
        // ... but the next reading confirms it (within step of the prior raw).
        assert_eq!(upd(&mut f, 62.0), Some(62.0));
    }

    #[test]
    fn zero_humidity_is_always_dropped() {
        let mut f = FieldFilter::default();
        upd(&mut f, 22.0);
        // 0 % is below the hard floor → dropped, last good held ...
        assert_eq!(upd(&mut f, 0.0), Some(22.0));
        // ... even when repeated (a stuck dead sensor never "confirms" 0 %).
        assert_eq!(upd(&mut f, 0.0), Some(22.0));
        assert_eq!(upd(&mut f, 0.0), Some(22.0));
        assert_eq!(upd(&mut f, 0.0), Some(22.0));
    }

    #[test]
    fn out_of_range_high_is_dropped() {
        let mut f = FieldFilter::default();
        upd(&mut f, 50.0);
        assert_eq!(upd(&mut f, 150.0), Some(50.0));
    }

    #[test]
    fn sustained_drift_resyncs_after_max_rejects() {
        let mut f = FieldFilter::default();
        upd(&mut f, 20.0);
        // A jumpy in-range ramp that steps > max_step each time and never lands
        // within step of the prior raw: rejected until the resync escape fires.
        assert_eq!(upd(&mut f, 35.0), Some(20.0)); // reject 1
        assert_eq!(upd(&mut f, 52.0), Some(20.0)); // reject 2
        assert_eq!(upd(&mut f, 70.0), Some(20.0)); // reject 3
        assert_eq!(upd(&mut f, 88.0), Some(88.0)); // resync
    }
}
