//! 24 h temperature history — three RAM-only ring buffers of 1440 buckets,
//! each holding min/max over a 1-minute window. Sampled once per second from
//! the main loop. The slots are heap-allocated (a 1440-slot array would be too
//! large to build on the stack). The OLED downsamples to its 128 px width; the
//! web shows the full resolution.

use crate::cwl_data::CwlData;

pub const HISTORY_SLOTS: usize = 1440;
pub const BUCKET_MS: u32 = (24 * 60 * 60 * 1000 / HISTORY_SLOTS) as u32; // 60_000 ms (1 min)
pub const SAMPLE_INTERVAL_MS: u32 = 1_000;

/// Number of (time-downsampled) points in the retained MQTT snapshot. Kept well
/// below the full slot count so the payload fits one MQTT buffer and parses
/// without building a large value tree. Recovery re-bins by timestamp, so this
/// is independent of the live resolution.
pub const MQTT_SNAPSHOT_POINTS: usize = 220;

/// Humidity changes slowly, so its history is coarser: 288 buckets / 5 min.
pub const HUMIDITY_SLOTS: usize = 288;
pub const HUMIDITY_BUCKET_MS: u32 = (24 * 60 * 60 * 1000 / HUMIDITY_SLOTS) as u32; // 300_000 (5 min)

/// Downsampled humidity points in the retained MQTT snapshot. Fewer than the
/// slot count (humidity moves slowly) to keep the combined snapshot inside the
/// MQTT buffer alongside the temperature points.
pub const MQTT_HUMIDITY_POINTS: usize = 100;

#[derive(Debug, Clone, Copy)]
pub struct Bucket {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug)]
pub struct Channel {
    pub slots: Box<[Option<Bucket>]>, // length HISTORY_SLOTS (heap-allocated)
    pub head: usize,
}

impl Channel {
    pub fn new() -> Self {
        Self::with_len(HISTORY_SLOTS)
    }

    /// Create a channel with an explicit slot count (temperature uses
    /// `HISTORY_SLOTS`, humidity uses the coarser `HUMIDITY_SLOTS`).
    pub fn with_len(len: usize) -> Self {
        Self { slots: vec![None; len].into_boxed_slice(), head: 0 }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Fold a sample into the current head slot. Initialises the slot on the
    /// first sample of a new bucket, updates min/max otherwise.
    pub fn fold(&mut self, value: f32) {
        match self.slots[self.head] {
            None => self.slots[self.head] = Some(Bucket { min: value, max: value }),
            Some(ref mut b) => {
                if value < b.min { b.min = value; }
                if value > b.max { b.max = value; }
            }
        }
    }

    /// Advance head to the next slot, clearing it so the rolling window
    /// discards the bucket that's about to be reused.
    pub fn advance(&mut self) {
        self.head = (self.head + 1) % self.len();
        self.slots[self.head] = None;
    }

    /// Aggregate min/max across all populated slots. Returns `None` if no
    /// slot has data yet (cold start).
    pub fn min_max(&self) -> Option<(f32, f32)> {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        let mut any = false;
        for slot in self.slots.iter() {
            if let Some(b) = slot {
                if b.min < lo { lo = b.min; }
                if b.max > hi { hi = b.max; }
                any = true;
            }
        }
        if any { Some((lo, hi)) } else { None }
    }

    /// Read slots in display order: column 0 = oldest, column HISTORY_SLOTS-1 =
    /// newest. Newest is the slot we're currently aggregating into (head).
    pub fn slot_at_column(&self, col: usize) -> Option<Bucket> {
        let n = self.len();
        let idx = (self.head + n - (n - 1 - col)) % n;
        self.slots[idx]
    }

    /// Aggregate min/max over a half-open range of display columns `[lo, hi)`.
    pub fn range_minmax(&self, lo: usize, hi: usize) -> Option<Bucket> {
        let mut bmin = f32::INFINITY;
        let mut bmax = f32::NEG_INFINITY;
        let mut any = false;
        for col in lo..hi.min(self.len()) {
            if let Some(b) = self.slot_at_column(col) {
                if b.min < bmin { bmin = b.min; }
                if b.max > bmax { bmax = b.max; }
                any = true;
            }
        }
        if any { Some(Bucket { min: bmin, max: bmax }) } else { None }
    }

    /// Downsample to `out_width` columns: returns the aggregated bucket for
    /// output column `out_col`. Used by the OLED (128 px) over the full history.
    pub fn aggregated_column(&self, out_col: usize, out_width: usize) -> Option<Bucket> {
        let n = self.len();
        let lo = out_col * n / out_width;
        let hi = ((out_col + 1) * n / out_width).max(lo + 1);
        self.range_minmax(lo, hi)
    }

    /// Append the channel as a JSON array of `[min,max]` / `null`, oldest→newest,
    /// directly into `out` — no intermediate allocation, so the caller can build
    /// a large payload without holding several big temporary strings at once.
    pub fn append_json_array(&self, out: &mut String) {
        self.append_downsampled(out, self.len());
    }

    /// Like [`append_json_array`] but aggregated down to `out_width` columns
    /// (oldest→newest). Used to shrink the web `/api/history` payload — at full
    /// 1440-slot resolution the JSON is large enough to risk exhausting the heap.
    pub fn append_downsampled(&self, out: &mut String, out_width: usize) {
        out.push('[');
        for col in 0..out_width {
            if col > 0 { out.push(','); }
            match self.aggregated_column(col, out_width) {
                Some(b) => out.push_str(&format!("[{:.2},{:.2}]", b.min, b.max)),
                None => out.push_str("null"),
            }
        }
        out.push(']');
    }

    /// Forward-fill `None` gaps between the first and last populated slot so a
    /// sparsely-restored channel renders as a continuous line. Assumes `head`
    /// is the newest slot (slot index == display column), as set by restore.
    pub fn fill_gaps_forward(&mut self) {
        let mut last: Option<Bucket> = None;
        let mut started = false;
        for i in 0..self.len() {
            match self.slots[i] {
                Some(b) => { last = Some(b); started = true; }
                None => { if started { self.slots[i] = last; } }
            }
        }
    }
}

/// Synthetic-summer-day temperature snapshot used by both the dev-mode
/// history preload and the OpenTherm simulator so the chart shows one
/// continuous curve across the preload→live boundary.
#[cfg(feature = "simulate-ot")]
pub struct SimDayCycle {
    pub outdoor: f32,
    pub indoor: f32,
}

#[cfg(feature = "simulate-ot")]
pub const SIM_PEAK_HOUR: f32 = 14.0;

/// Evaluate the simulated summer-day cycle. `hours_offset` is hours since
/// boot (use negative values for the preload to project backwards). At
/// offset 0 the firmware boots at `SIM_PEAK_HOUR` (14:00 — outdoor peak).
#[cfg(feature = "simulate-ot")]
pub fn simulated_day_cycle(hours_offset: f32) -> SimDayCycle {
    use core::f32::consts::PI;
    let outdoor_base = 20.0_f32;
    let outdoor_amp = 8.0_f32;
    let indoor_base = 22.0_f32;
    let indoor_amp = 0.5_f32;

    let hour_of_day = (SIM_PEAK_HOUR + hours_offset).rem_euclid(24.0);
    let phase = (hour_of_day - SIM_PEAK_HOUR) * (2.0 * PI / 24.0);
    let outdoor = outdoor_base + outdoor_amp * phase.cos();
    let indoor = indoor_base + indoor_amp * (phase + PI / 4.0).sin();
    SimDayCycle { outdoor, indoor }
}

#[derive(Debug)]
pub struct TempHistory {
    pub outdoor: Channel,
    pub indoor: Channel,
    // Coarse humidity history (RH %), independent 5-min buckets.
    pub indoor_humidity: Channel,
    pub outdoor_humidity: Channel,
    bucket_start_ms: u32,
    last_sample_ms: u32,
    humidity_bucket_start_ms: u32,
}

impl TempHistory {
    pub fn new() -> Self {
        let h = Self {
            outdoor: Channel::new(),
            indoor: Channel::new(),
            indoor_humidity: Channel::with_len(HUMIDITY_SLOTS),
            outdoor_humidity: Channel::with_len(HUMIDITY_SLOTS),
            bucket_start_ms: 0,
            last_sample_ms: 0,
            humidity_bucket_start_ms: 0,
        };
        #[cfg(feature = "simulate-ot")]
        return Self::preloaded_simulation(h);
        #[cfg(not(feature = "simulate-ot"))]
        h
    }

    /// Backfill 24 h of synthetic data so the dev-mode chart pages have
    /// something interesting on boot. Column 127 ("now") is anchored at
    /// `SIM_PEAK_HOUR`; `simulated_day_cycle` produces the exact same curve
    /// that `ot_master::simulate` uses, so the preload and the live
    /// samples form a continuous trace.
    #[cfg(feature = "simulate-ot")]
    fn preloaded_simulation(mut h: Self) -> Self {
        let slot_hours: f32 = 24.0 / HISTORY_SLOTS as f32;
        for col in 0..HISTORY_SLOTS {
            let hours_ago = (HISTORY_SLOTS - 1 - col) as f32 * slot_hours;
            let cycle = simulated_day_cycle(-hours_ago);
            // Column-to-slot mapping mirrors Channel::slot_at_column with head=0.
            let idx = (HISTORY_SLOTS + col + 1) % HISTORY_SLOTS;
            h.outdoor.slots[idx] = Some(Bucket { min: cycle.outdoor, max: cycle.outdoor });
            h.indoor.slots[idx] = Some(Bucket { min: cycle.indoor, max: cycle.indoor });
        }
        h
    }

    /// Fold the latest temperatures into the active bucket. Throttled to
    /// `SAMPLE_INTERVAL_MS`. Returns `true` when a bucket boundary was
    /// crossed (so the display can be marked dirty).
    pub fn sample(&mut self, now_ms: u32, data: &CwlData) -> bool {
        if self.last_sample_ms != 0
            && now_ms.wrapping_sub(self.last_sample_ms) < SAMPLE_INTERVAL_MS
        {
            return false;
        }
        self.last_sample_ms = now_ms;

        let mut rolled = false;
        if self.bucket_start_ms == 0 {
            self.bucket_start_ms = now_ms;
        } else if now_ms.wrapping_sub(self.bucket_start_ms) >= BUCKET_MS {
            self.outdoor.advance();
            self.indoor.advance();
            self.bucket_start_ms = now_ms;
            rolled = true;
        }

        // Only fold real readings. `connected` can go true a moment before the
        // first ID-80/82 temperature is actually read, so the value still reads
        // 0.0 — and the fields default to 0.0 before the link is up. Folding
        // those would peg the bucket's min at 0 °C and drag the chart down, so
        // we reject the 0.0 sentinel (and implausible values). The bucket still
        // advances on schedule above; it just stays empty until valid data.
        if data.connected {
            let s = data.supply_temp;
            let e = data.exhaust_temp;
            if is_real_temp(s) { self.outdoor.fold(s); }
            if is_real_temp(e) { self.indoor.fold(e); }
        }

        rolled
    }

    /// Fold the aggregated indoor/outdoor humidity (max indoor RH, outdoor RH)
    /// into the coarse humidity channels. Independent 5-min buckets; `None`
    /// values (no fresh sensor) simply aren't folded. Cheap to call each second.
    pub fn sample_humidity(&mut self, now_ms: u32, indoor_rh: Option<f32>, outdoor_rh: Option<f32>) {
        if self.humidity_bucket_start_ms == 0 {
            self.humidity_bucket_start_ms = now_ms;
        } else if now_ms.wrapping_sub(self.humidity_bucket_start_ms) >= HUMIDITY_BUCKET_MS {
            self.indoor_humidity.advance();
            self.outdoor_humidity.advance();
            self.humidity_bucket_start_ms = now_ms;
        }
        if let Some(rh) = indoor_rh { self.indoor_humidity.fold(rh); }
        if let Some(rh) = outdoor_rh { self.outdoor_humidity.fold(rh); }
    }

    /// Build the JSON payload for the web `/api/history` endpoint. The
    /// temperature channels are emitted at HALF resolution (720 columns); at the
    /// full 1440-slot resolution the JSON was large enough that building it could
    /// exhaust the heap and crash the request. Built into a single pre-sized
    /// buffer with each array appended in place (no `format!`/intermediate copies).
    pub fn full_json(&self, now_epoch: i64, events_json: &str) -> String {
        // Half the temperature resolution: 720 columns over the 24 h window.
        let web_slots = HISTORY_SLOTS / 2;
        let web_bucket_ms = BUCKET_MS * (HISTORY_SLOTS / web_slots) as u32; // 2 min
        let mut s = String::with_capacity((web_slots * 2 + HUMIDITY_SLOTS * 2) * 14 + events_json.len() + 256);
        s.push_str("{\"slots\":");
        s.push_str(&web_slots.to_string());
        s.push_str(",\"bucketMs\":");
        s.push_str(&web_bucket_ms.to_string());
        s.push_str(",\"nowEpoch\":");
        s.push_str(&now_epoch.to_string());
        s.push_str(",\"supply\":");
        self.outdoor.append_downsampled(&mut s, web_slots);
        s.push_str(",\"exhaust\":");
        self.indoor.append_downsampled(&mut s, web_slots);
        s.push_str(",\"humiditySlots\":");
        s.push_str(&HUMIDITY_SLOTS.to_string());
        s.push_str(",\"humidityBucketMs\":");
        s.push_str(&HUMIDITY_BUCKET_MS.to_string());
        s.push_str(",\"indoorHumidity\":");
        self.indoor_humidity.append_json_array(&mut s);
        s.push_str(",\"outdoorHumidity\":");
        self.outdoor_humidity.append_json_array(&mut s);
        s.push_str(",\"events\":");
        s.push_str(events_json);
        s.push('}');
        s
    }

    /// Build the retained MQTT snapshot: a time-downsampled list of timestamped
    /// points `[epoch, sMin,sMax, eMin,eMax]` (null where a channel is missing).
    /// Because each point carries its own epoch, recovery re-bins by time and is
    /// independent of the live resolution.
    pub fn snapshot_json(&self, now_epoch: i64) -> String {
        let bsec = (BUCKET_MS / 1000) as i64;
        let newest = HISTORY_SLOTS - 1;
        let mut s = String::with_capacity(MQTT_SNAPSHOT_POINTS * 52 + 32);
        s.push_str("{\"interval\":");
        s.push_str(&bsec.to_string());
        s.push_str(",\"points\":[");
        let mut first = true;
        for j in 0..MQTT_SNAPSHOT_POINTS {
            let lo = j * HISTORY_SLOTS / MQTT_SNAPSHOT_POINTS;
            let hi = ((j + 1) * HISTORY_SLOTS / MQTT_SNAPSHOT_POINTS).max(lo + 1);
            let sup = match self.outdoor.range_minmax(lo, hi) { Some(b) => b, None => continue };
            let exh = self.indoor.range_minmax(lo, hi);
            // Representative time = newest column in the group.
            let rep_col = hi.min(HISTORY_SLOTS) - 1;
            let t = now_epoch - ((newest - rep_col) as i64) * bsec;
            if !first { s.push(','); }
            first = false;
            s.push('[');
            s.push_str(&t.to_string());
            push_pair(&mut s, Some(sup));
            push_pair(&mut s, exh);
            s.push(']');
        }
        s.push(']');

        // Humidity history (coarser; may be empty if no sensors are configured).
        // Emitted as a separate epoch-stamped point list so recovery re-bins it
        // on its own 5-minute grid, independent of the temperature resolution.
        let hbsec = (HUMIDITY_BUCKET_MS / 1000) as i64;
        let hnewest = HUMIDITY_SLOTS - 1;
        s.push_str(",\"humInterval\":");
        s.push_str(&hbsec.to_string());
        s.push_str(",\"humPoints\":[");
        let mut hfirst = true;
        for j in 0..MQTT_HUMIDITY_POINTS {
            let lo = j * HUMIDITY_SLOTS / MQTT_HUMIDITY_POINTS;
            let hi = ((j + 1) * HUMIDITY_SLOTS / MQTT_HUMIDITY_POINTS).max(lo + 1);
            let ind = self.indoor_humidity.range_minmax(lo, hi);
            let outd = self.outdoor_humidity.range_minmax(lo, hi);
            if ind.is_none() && outd.is_none() { continue; }
            let rep_col = hi.min(HUMIDITY_SLOTS) - 1;
            let t = now_epoch - ((hnewest - rep_col) as i64) * hbsec;
            if !hfirst { s.push(','); }
            hfirst = false;
            s.push('[');
            s.push_str(&t.to_string());
            push_pair(&mut s, ind);
            push_pair(&mut s, outd);
            s.push(']');
        }
        s.push_str("]}");
        s
    }

    /// Restore from a snapshot produced by [`snapshot_json`]. Each point is
    /// re-binned into the live grid by its timestamp relative to `now_epoch`, so
    /// this works even if the bucket size changed since the snapshot was written.
    /// Sparse points are forward-filled into a continuous line. Parsed by hand
    /// (no value tree) to stay memory-cheap. Malformed input is a no-op.
    pub fn restore_from_snapshot(&mut self, json: &[u8], now_epoch: i64) -> bool {
        let text = match std::str::from_utf8(json) { Ok(t) => t, Err(_) => return false };
        let pidx = match text.find("\"points\"") { Some(i) => i, None => return false };
        let arr_start = match text[pidx..].find('[') { Some(i) => pidx + i, None => return false };
        // Temperature points end where the (optional) humidity list begins; bound
        // the temp scan there so it doesn't ingest humidity points as temps.
        let temp_end = text.find("\"humPoints\"").unwrap_or(text.len());

        let bsec = (BUCKET_MS / 1000) as i64;
        let newest = (HISTORY_SLOTS - 1) as i64;

        // Reset both channels with head at the newest slot (slot idx == col).
        for ch in [&mut self.outdoor, &mut self.indoor] {
            ch.head = HISTORY_SLOTS - 1;
            for slot in ch.slots.iter_mut() { *slot = None; }
        }

        let mut any = false;
        let mut i = arr_start + 1;
        while let Some(rel) = text[i..].find('[') {
            let open = i + rel;
            if open >= temp_end { break; } // reached the humidity list
            let close = match text[open..].find(']') { Some(c) => open + c, None => break };
            let inner = &text[open + 1..close];
            let mut it = inner.split(',');
            let t = it.next().and_then(|x| x.trim().parse::<i64>().ok());
            let s_min = parse_opt(it.next());
            let s_max = parse_opt(it.next());
            let e_min = parse_opt(it.next());
            let e_max = parse_opt(it.next());
            if let Some(t) = t {
                // Bucket index back from "now", rounded.
                let back = (now_epoch - t + bsec / 2) / bsec;
                let col = newest - back;
                if col >= 0 && col < HISTORY_SLOTS as i64 {
                    let col = col as usize;
                    if let (Some(a), Some(b)) = (s_min, s_max) {
                        self.outdoor.slots[col] = Some(Bucket { min: a, max: b }); any = true;
                    }
                    if let (Some(a), Some(b)) = (e_min, e_max) {
                        self.indoor.slots[col] = Some(Bucket { min: a, max: b });
                    }
                }
            }
            i = close + 1;
        }

        if any {
            self.outdoor.fill_gaps_forward();
            self.indoor.fill_gaps_forward();
            // Continue sampling into the restored newest bucket.
            self.bucket_start_ms = 0;
            self.last_sample_ms = 0;
        }

        // Humidity points (optional — absent in pre-humidity snapshots). Re-binned
        // on the coarser 5-minute grid by their own epochs.
        if let Some(hpidx) = text.find("\"humPoints\"") {
            if let Some(harr) = text[hpidx..].find('[').map(|x| hpidx + x) {
                let hbsec = (HUMIDITY_BUCKET_MS / 1000) as i64;
                let hnewest = (HUMIDITY_SLOTS - 1) as i64;
                for ch in [&mut self.indoor_humidity, &mut self.outdoor_humidity] {
                    ch.head = HUMIDITY_SLOTS - 1;
                    for slot in ch.slots.iter_mut() { *slot = None; }
                }
                let mut hany = false;
                let mut k = harr + 1;
                while let Some(rel) = text[k..].find('[') {
                    let open = k + rel;
                    let close = match text[open..].find(']') { Some(c) => open + c, None => break };
                    let inner = &text[open + 1..close];
                    let mut it = inner.split(',');
                    let t = it.next().and_then(|x| x.trim().parse::<i64>().ok());
                    let in_min = parse_opt(it.next());
                    let in_max = parse_opt(it.next());
                    let out_min = parse_opt(it.next());
                    let out_max = parse_opt(it.next());
                    if let Some(t) = t {
                        let back = (now_epoch - t + hbsec / 2) / hbsec;
                        let col = hnewest - back;
                        if col >= 0 && col < HUMIDITY_SLOTS as i64 {
                            let col = col as usize;
                            if let (Some(a), Some(b)) = (in_min, in_max) {
                                self.indoor_humidity.slots[col] = Some(Bucket { min: a, max: b }); hany = true;
                            }
                            if let (Some(a), Some(b)) = (out_min, out_max) {
                                self.outdoor_humidity.slots[col] = Some(Bucket { min: a, max: b }); hany = true;
                            }
                        }
                    }
                    k = close + 1;
                }
                if hany {
                    self.indoor_humidity.fill_gaps_forward();
                    self.outdoor_humidity.fill_gaps_forward();
                    self.humidity_bucket_start_ms = 0;
                }
            }
        }
        any
    }
}

/// A plausible temperature reading: rejects the 0.0 "not yet read" sentinel and
/// out-of-range values. A genuine 0 °C is sacrificed (rare, and only the exact
/// sentinel band) to keep boot-time zeros out of the history.
fn is_real_temp(t: f32) -> bool {
    t.abs() > 0.05 && t > -60.0 && t < 90.0
}

/// Append `,min,max` (2-decimal) or `,null,null` for a channel bucket.
fn push_pair(s: &mut String, b: Option<Bucket>) {
    match b {
        Some(b) => s.push_str(&format!(",{:.2},{:.2}", b.min, b.max)),
        None => s.push_str(",null,null"),
    }
}

/// Parse one comma-token as `Some(f32)` or `None` (for `null` / missing / bad).
fn parse_opt(tok: Option<&str>) -> Option<f32> {
    match tok {
        Some(t) => {
            let t = t.trim().trim_end_matches(']');
            if t == "null" || t.is_empty() { None } else { t.parse::<f32>().ok() }
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_two_samples_updates_min_max() {
        let mut ch = Channel::new();
        ch.fold(20.0);
        ch.fold(15.0);
        ch.fold(25.0);
        let b = ch.slots[ch.head].unwrap();
        assert!((b.min - 15.0).abs() < 1e-6);
        assert!((b.max - 25.0).abs() < 1e-6);
    }

    #[test]
    fn advance_then_fold_starts_fresh_bucket() {
        let mut ch = Channel::new();
        ch.fold(20.0);
        ch.advance();
        ch.fold(10.0);
        let b = ch.slots[ch.head].unwrap();
        assert!((b.min - 10.0).abs() < 1e-6);
        assert!((b.max - 10.0).abs() < 1e-6);
    }

    #[test]
    fn min_max_over_all_none_returns_none() {
        let ch = Channel::new();
        assert!(ch.min_max().is_none());
    }

    #[test]
    fn min_max_aggregates_populated_slots() {
        let mut ch = Channel::new();
        ch.fold(18.0);
        ch.advance();
        ch.fold(22.0);
        ch.advance();
        ch.fold(15.0);
        let (lo, hi) = ch.min_max().unwrap();
        assert!((lo - 15.0).abs() < 1e-6);
        assert!((hi - 22.0).abs() < 1e-6);
    }

    #[test]
    fn slot_at_column_returns_oldest_to_newest() {
        let mut ch = Channel::new();
        // Slot 0 = first bucket = will become the oldest after we advance.
        ch.fold(1.0);
        ch.advance();
        ch.fold(2.0);
        ch.advance();
        ch.fold(3.0);
        // head now points at the slot holding 3.0; the last column is newest.
        let n = HISTORY_SLOTS - 1;
        assert!((ch.slot_at_column(n).unwrap().min - 3.0).abs() < 1e-6);
        assert!((ch.slot_at_column(n - 1).unwrap().min - 2.0).abs() < 1e-6);
        assert!((ch.slot_at_column(n - 2).unwrap().min - 1.0).abs() < 1e-6);
        // Anything further back is still empty.
        assert!(ch.slot_at_column(n - 3).is_none());
        assert!(ch.slot_at_column(0).is_none());
    }

    #[test]
    fn snapshot_roundtrip_restores_recent_data() {
        let mut h = TempHistory::new();
        // Two newest buckets per channel.
        h.outdoor.fold(18.0); h.indoor.fold(21.0);
        h.outdoor.advance(); h.indoor.advance();
        h.outdoor.fold(19.5); h.indoor.fold(20.5);

        let now = 1_700_000_000;
        let json = h.snapshot_json(now);
        let mut h2 = TempHistory::new();
        assert!(h2.restore_from_snapshot(json.as_bytes(), now));

        // The newest column carries the (downsampled) recent values back.
        let newest = HISTORY_SLOTS - 1;
        let o = h2.outdoor.slot_at_column(newest).unwrap();
        assert!((o.max - 19.5).abs() < 0.05);
        assert!((o.min - 18.0).abs() < 0.05);
        let i = h2.indoor.slot_at_column(newest).unwrap();
        assert!((i.max - 21.0).abs() < 0.05);
    }

    #[test]
    fn restore_ignores_malformed_input() {
        let mut h = TempHistory::new();
        let now = 1_700_000_000;
        assert!(!h.restore_from_snapshot(b"not json at all", now));
        assert!(!h.restore_from_snapshot(b"{}", now));
    }
}
