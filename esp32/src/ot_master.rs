//! OpenTherm master — polling state machine for Wolf CWL communication.

use std::sync::{Arc, Mutex};
use log::info;
#[cfg(not(feature = "simulate-ot"))]
use log::warn;

use crate::app_state::AppStateInner;
#[cfg(not(feature = "simulate-ot"))]
use crate::cwl_data::{decode_f88, TSP_MAX_INDEX};

type AppState = Arc<Mutex<AppStateInner>>;

const POLL_INTERVAL_MS: u32 = 1000;
#[cfg(not(feature = "simulate-ot"))]
const DISCONNECT_TIMEOUT_MS: u32 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum PollState {
    MasterConfig,
    Status,
    Setpoint,
    Config,
    Ventilation,
    SupplyTemp,
    ExhaustTemp,
    MasterVersion,
    SlaveVersion,
    Tsp,
    Fault,
    SupplyOutlet,
    ExhaustOutlet,
    ExhaustFan,
    SupplyFan,
    CycleDone,
}

#[allow(dead_code)]
pub struct OtMaster {
    #[cfg(not(feature = "simulate-ot"))]
    ot: crate::opentherm_ffi::OpenTherm,
    state: AppState,
    poll_state: PollState,
    last_poll_ms: u32,
    tsp_index: u8,
    probe_complete: bool,
    filter_reset_active: bool,
    // Diagnostic counters — dumped via a periodic info log so the user can
    // correlate exact poll outcomes against a Saleae capture.
    stat_success: u32,
    stat_invalid: u32,
    stat_timeout: u32,
    last_stats_log_ms: u32,
}

impl OtMaster {
    #[cfg(not(feature = "simulate-ot"))]
    pub fn new(state: AppState, in_pin: i32, out_pin: i32) -> Result<Self, &'static str> {
        let ot = crate::opentherm_ffi::OpenTherm::new(in_pin, out_pin)?;
        info!("OpenTherm: Initialized (IN={}, OUT={})", in_pin, out_pin);
        Ok(Self {
            ot,
            state,
            poll_state: PollState::MasterConfig,
            last_poll_ms: 0,
            tsp_index: 0,
            probe_complete: false,
            filter_reset_active: false,
            stat_success: 0,
            stat_invalid: 0,
            stat_timeout: 0,
            last_stats_log_ms: 0,
        })
    }

    #[cfg(feature = "simulate-ot")]
    pub fn new(state: AppState, _in_pin: i32, _out_pin: i32) -> Result<Self, &'static str> {
        info!("OpenTherm: Simulation mode");
        // No CWL to mirror in sim mode — mark level as known up front so the
        // Setpoint write is never skipped.
        state.lock().unwrap().initial_level_known = true;
        Ok(Self {
            state,
            poll_state: PollState::MasterConfig,
            last_poll_ms: 0,
            tsp_index: 0,
            probe_complete: true,
            filter_reset_active: false,
            stat_success: 0,
            stat_invalid: 0,
            stat_timeout: 0,
            last_stats_log_ms: 0,
        })
    }

    /// Run one poll/simulate step. Returns `true` if a step was actually
    /// executed this call (used by the OT thread to bump the display dirty
    /// flag only when something might have changed, instead of every 100 ms).
    pub fn update(&mut self, now_ms: u32) -> bool {
        if now_ms.wrapping_sub(self.last_poll_ms) < POLL_INTERVAL_MS {
            return false;
        }
        self.last_poll_ms = now_ms;

        // Boot fallback: if 30 s have passed and neither the CWL (via ID 77),
        // the scheduler, nor the user has supplied a level, default to Normal
        // so the unit doesn't sit indefinitely with our Setpoint write skipped.
        const BOOT_LEVEL_FALLBACK_MS: u32 = 30_000;
        if now_ms >= BOOT_LEVEL_FALLBACK_MS {
            let mut st = self.state.lock().unwrap();
            if !st.initial_level_known {
                st.requested_vent_level = 2; // Normal
                st.initial_level_known = true;
                info!("OT: boot fallback — no CWL level seen in 30s, defaulting to Normal");
            }
        }

        #[cfg(feature = "simulate-ot")]
        {
            self.simulate(now_ms);
            return true;
        }

        #[cfg(not(feature = "simulate-ot"))]
        {
            self.poll_cycle(now_ms);
            true
        }
    }

    #[cfg(not(feature = "simulate-ot"))]
    fn poll_cycle(&mut self, now_ms: u32) {
        use crate::opentherm_ffi::{MessageType, OpenTherm, ResponseStatus};

        // Probe additional IDs once connected
        if !self.probe_complete {
            let connected = self.state.lock().unwrap().cwl_data.connected;
            if connected {
                self.probe_additional_ids();
                return;
            }
        }

        // Consume pending filter-reset request (set by MQTT / web). The Status
        // handler below will OR bit 4 into the next ID=70 request and clear
        // the one-shot, so the filter-timer reset is sent exactly once.
        {
            let mut st = self.state.lock().unwrap();
            if st.requested_filter_reset {
                st.requested_filter_reset = false;
                self.filter_reset_active = true;
            }
        }

        let st = self.state.lock().unwrap();

        let (request, process_fn): (u32, Box<dyn FnOnce(u32, &mut AppStateInner)>) = match self.poll_state {
            PollState::MasterConfig => {
                // Write-Data ID=2 value=0x0012 (master config, member-id=18)
                let req = OpenTherm::build_request(MessageType::WriteData, 2, 0x0012);
                self.poll_state = PollState::Status;
                (req, Box::new(|_, _| {}))
            }
            PollState::Status => {
                let mut flags: u8 = 0x01; // ventilation enable
                if st.requested_bypass_open { flags |= 0x02; }
                if self.filter_reset_active { flags |= 0x10; }
                let req = OpenTherm::build_request(MessageType::ReadData, 70, (flags as u16) << 8);
                if self.filter_reset_active {
                    self.filter_reset_active = false;
                }
                self.poll_state = PollState::Setpoint;
                (req, Box::new(|resp, st| {
                    let hi = ((resp >> 8) & 0xFF) as u8;
                    let lo = (resp & 0xFF) as u8;
                    st.cwl_data.fault = hi & 0x01 != 0;
                    st.cwl_data.ventilation_active = (hi >> 1) & 0x01 != 0;
                    st.cwl_data.cooling_active = (hi >> 2) & 0x01 != 0;
                    st.cwl_data.dhw_active = (hi >> 3) & 0x01 != 0;
                    // Filter signal sources, in order of reliability for this
                    // hardware:
                    //   * Status LO bit 5 — empirical, confirmed across BM
                    //     and PCB captures. This is the OEM channel Wolf
                    //     (Brink / Viessmann) uses.
                    //   * Status HI bit 4 — OpenTherm V/H spec filter bit.
                    //     Wolf doesn't set it but we still honour it for
                    //     spec-compliant units.
                    //   * Fault OEM code 6 — earlier hypothesis from one
                    //     capture (FIL2); kept as a fallback though we
                    //     no longer trust it as the primary signal.
                    let filter_bit_hi = (hi >> 4) & 0x01 != 0;
                    let filter_bit_lo = (lo >> 5) & 0x01 != 0;
                    let was_dirty = st.cwl_data.filter_dirty;
                    st.cwl_data.filter_dirty = filter_bit_lo
                        || filter_bit_hi
                        || st.cwl_data.oem_fault_code == 6;
                    st.cwl_data.diag_event = (hi >> 5) & 0x01 != 0;
                    if st.cwl_data.filter_dirty != was_dirty {
                        info!(
                            "OT: filter_dirty {} (status HI=0x{:02X} LO=0x{:02X}, oem_fault_code={})",
                            st.cwl_data.filter_dirty, hi, lo, st.cwl_data.oem_fault_code
                        );
                    }
                }))
            }
            PollState::Setpoint => {
                // Until we've inferred the CWL's actual level from ID 77 (or
                // the user explicitly chose one), skip the WRITE so we don't
                // override an already-running unit with a stale config value.
                if !st.initial_level_known {
                    self.poll_state = PollState::Config;
                    return;
                }
                let level = st.requested_vent_level.min(3);
                let req = OpenTherm::build_request(MessageType::WriteData, 71, level as u16);
                self.poll_state = PollState::Config;
                (req, Box::new(|resp, st| {
                    st.cwl_data.ventilation_level = (resp & 0xFF) as u8;
                }))
            }
            PollState::Config => {
                let req = OpenTherm::build_request(MessageType::ReadData, 74, 0);
                self.poll_state = PollState::Ventilation;
                (req, Box::new(|resp, st| {
                    st.cwl_data.slave_flags = ((resp >> 8) & 0xFF) as u8;
                    st.cwl_data.slave_member_id = (resp & 0xFF) as u8;
                }))
            }
            PollState::Ventilation => {
                let req = OpenTherm::build_request(MessageType::ReadData, 77, 0);
                self.poll_state = PollState::SupplyTemp;
                (req, Box::new(|resp, st| {
                    // ID 77 is u8(%) in the LO byte per the OT V/H spec —
                    // reading the HI byte always returned 0.
                    let pct = (resp & 0xFF) as u8;
                    st.cwl_data.relative_ventilation = pct;
                    // First reading: mirror the CWL's actual level into the
                    // requested state so we don't change anything when we
                    // start writing ID 71. Map by closest documented %:
                    //   0% → Off (0), 51% → Reduced (1),
                    //   67% → Normal (2), 100% → Party (3).
                    if !st.initial_level_known {
                        let level = if pct >= 84 { 3 }
                            else if pct >= 59 { 2 }
                            else if pct >= 26 { 1 }
                            else { 0 };
                        st.requested_vent_level = level;
                        st.initial_level_known = true;
                        info!("OT: inferred initial vent level={} from {}%", level, pct);
                    }
                }))
            }
            PollState::SupplyTemp => {
                let req = OpenTherm::build_request(MessageType::ReadData, 80, 0);
                self.poll_state = PollState::ExhaustTemp;
                (req, Box::new(|resp, st| {
                    st.cwl_data.supply_inlet_temp = decode_f88((resp & 0xFFFF) as u16);
                }))
            }
            PollState::ExhaustTemp => {
                let req = OpenTherm::build_request(MessageType::ReadData, 82, 0);
                self.poll_state = PollState::MasterVersion;
                (req, Box::new(|resp, st| {
                    st.cwl_data.exhaust_inlet_temp = decode_f88((resp & 0xFFFF) as u16);
                }))
            }
            PollState::MasterVersion => {
                let req = OpenTherm::build_request(MessageType::WriteData, 126, 0x1202);
                self.poll_state = PollState::SlaveVersion;
                (req, Box::new(|_, _| {}))
            }
            PollState::SlaveVersion => {
                let req = OpenTherm::build_request(MessageType::ReadData, 127, 0);
                self.poll_state = PollState::Tsp;
                (req, Box::new(|resp, st| {
                    st.cwl_data.slave_type = ((resp >> 8) & 0xFF) as u8;
                    st.cwl_data.slave_version = (resp & 0xFF) as u8;
                }))
            }
            PollState::Tsp => {
                let req = OpenTherm::build_request(MessageType::ReadData, 89, (self.tsp_index as u16) << 8);
                self.tsp_index = (self.tsp_index + 1) % TSP_MAX_INDEX as u8;
                self.poll_state = PollState::Fault;
                (req, Box::new(|resp, st| {
                    let index = ((resp >> 8) & 0xFF) as usize;
                    let value = (resp & 0xFF) as u8;
                    if index < TSP_MAX_INDEX {
                        st.cwl_data.tsp_values[index] = value;
                        st.cwl_data.tsp_valid[index] = true;
                        st.cwl_data.update_derived_values();
                    }
                }))
            }
            PollState::Fault => {
                let req = OpenTherm::build_request(MessageType::ReadData, 72, 0);
                // Next state depends on supported IDs
                self.poll_state = if st.cwl_data.supports_id81 { PollState::SupplyOutlet }
                    else if st.cwl_data.supports_id83 { PollState::ExhaustOutlet }
                    else if st.cwl_data.supports_id84 { PollState::ExhaustFan }
                    else if st.cwl_data.supports_id85 { PollState::SupplyFan }
                    else { PollState::CycleDone };
                (req, Box::new(|resp, st| {
                    st.cwl_data.fault_flags = ((resp >> 8) & 0xFF) as u8;
                    let code = (resp & 0xFF) as u8;
                    let prev_code = st.cwl_data.oem_fault_code;
                    st.cwl_data.oem_fault_code = code;
                    // Mirror filter detection here so a fresh OEM code is reflected
                    // immediately (the Status handler reads the value cached here).
                    if code == 6 {
                        st.cwl_data.filter_dirty = true;
                    }
                    if code != prev_code {
                        info!("OT: oem_fault_code {} -> {}", prev_code, code);
                    }
                }))
            }
            PollState::SupplyOutlet => {
                let req = OpenTherm::build_request(MessageType::ReadData, 81, 0);
                self.poll_state = if st.cwl_data.supports_id83 { PollState::ExhaustOutlet }
                    else if st.cwl_data.supports_id84 { PollState::ExhaustFan }
                    else if st.cwl_data.supports_id85 { PollState::SupplyFan }
                    else { PollState::CycleDone };
                (req, Box::new(|resp, st| {
                    st.cwl_data.supply_outlet_temp = decode_f88((resp & 0xFFFF) as u16);
                }))
            }
            PollState::ExhaustOutlet => {
                let req = OpenTherm::build_request(MessageType::ReadData, 83, 0);
                self.poll_state = if st.cwl_data.supports_id84 { PollState::ExhaustFan }
                    else if st.cwl_data.supports_id85 { PollState::SupplyFan }
                    else { PollState::CycleDone };
                (req, Box::new(|resp, st| {
                    st.cwl_data.exhaust_outlet_temp = decode_f88((resp & 0xFFFF) as u16);
                }))
            }
            PollState::ExhaustFan => {
                let req = OpenTherm::build_request(MessageType::ReadData, 84, 0);
                self.poll_state = if st.cwl_data.supports_id85 { PollState::SupplyFan } else { PollState::CycleDone };
                (req, Box::new(|resp, st| {
                    st.cwl_data.exhaust_fan_speed = (resp & 0xFFFF) as u16;
                }))
            }
            PollState::SupplyFan => {
                let req = OpenTherm::build_request(MessageType::ReadData, 85, 0);
                self.poll_state = PollState::CycleDone;
                (req, Box::new(|resp, st| {
                    st.cwl_data.supply_fan_speed = (resp & 0xFFFF) as u16;
                }))
            }
            PollState::CycleDone => {
                self.poll_state = PollState::MasterConfig;
                return;
            }
        };

        // Release lock before blocking I/O
        drop(st);

        // Send request (blocks ~1s max)
        let response = self.ot.send_request(request);
        let status = self.ot.last_status();

        let mut st = self.state.lock().unwrap();
        // Any non-Timeout status means the OT library saw something on the bus
        // (clean response or framing-glitched response). Bump the activity
        // counter so the status LED flashes either way.
        if status != ResponseStatus::Timeout {
            st.cwl_data.rx_seen_counter = st.cwl_data.rx_seen_counter.wrapping_add(1);
        }
        if status == ResponseStatus::Success {
            self.stat_success = self.stat_success.wrapping_add(1);
            st.cwl_data.connected = true;
            st.cwl_data.last_response_ms = now_ms;
            process_fn(response, &mut st);
        } else if status == ResponseStatus::Invalid {
            self.stat_invalid = self.stat_invalid.wrapping_add(1);
            // UNKNOWN_DATA_ID (msg type 7) is a spec-valid "I don't support this
            // ID" response — the OT library rejects it because it isn't a
            // READ_ACK/WRITE_ACK, but it's expected for IDs 126/127 on this
            // CWL. Only warn on genuine framing/parity glitches.
            let resp_msg_type = (response >> 28) & 0x7;
            let req_id = (request >> 16) & 0xFF;
            if resp_msg_type == 7 {
                // Quiet — known/expected. Counter still bumped so the LED
                // reflects the bus activity.
            } else {
                warn!(
                    "OpenTherm: invalid response — req_id={} req=0x{:08X} resp=0x{:08X}",
                    req_id, request, response
                );
            }
        } else if status == ResponseStatus::Timeout {
            self.stat_timeout = self.stat_timeout.wrapping_add(1);
            if st.cwl_data.connected && now_ms.wrapping_sub(st.cwl_data.last_response_ms) > DISCONNECT_TIMEOUT_MS {
                st.cwl_data.connected = false;
                info!("OpenTherm: CWL not responding");
            }
        }
        drop(st);

        // Periodic stats dump — every 30 s. Lets the user check whether
        // "missing LED flashes" match Timeouts (slave responded but library
        // never saw an edge) or something else, plus a snapshot of the
        // filter/fault decoding so a missing "Filter" overlay can be traced
        // back to the actual values the firmware decoded.
        if now_ms.wrapping_sub(self.last_stats_log_ms) >= 30_000 {
            self.last_stats_log_ms = now_ms;
            let total = self.stat_success.wrapping_add(self.stat_invalid).wrapping_add(self.stat_timeout);
            let st = self.state.lock().unwrap();
            info!(
                "OpenTherm stats: total={} success={} invalid={} timeout={} | fault={} filter_dirty={} oem_code={}",
                total, self.stat_success, self.stat_invalid, self.stat_timeout,
                st.cwl_data.fault, st.cwl_data.filter_dirty, st.cwl_data.oem_fault_code
            );
        }
    }

    #[cfg(not(feature = "simulate-ot"))]
    fn probe_additional_ids(&mut self) {
        use crate::opentherm_ffi::{MessageType, OpenTherm, ResponseStatus};

        info!("OpenTherm: Probing additional data IDs...");

        let probes: &[(u8, &str)] = &[
            (81, "Supply outlet temp"),
            (83, "Exhaust outlet temp"),
            (84, "Exhaust fan speed"),
            (85, "Supply fan speed"),
            (78, "Relative humidity"),
            (79, "CO2 level"),
            (87, "Nominal ventilation"),
            (88, "TSP count"),
        ];

        for &(id, name) in probes {
            let request = OpenTherm::build_request(MessageType::ReadData, id, 0);
            let response = self.ot.send_request(request);
            let status = self.ot.last_status();

            let supported = status == ResponseStatus::Success
                && OpenTherm::get_message_type(response) == MessageType::ReadAck;

            let mut st = self.state.lock().unwrap();
            match id {
                78 => st.cwl_data.supports_id78 = supported,
                79 => st.cwl_data.supports_id79 = supported,
                81 => st.cwl_data.supports_id81 = supported,
                83 => st.cwl_data.supports_id83 = supported,
                84 => st.cwl_data.supports_id84 = supported,
                85 => st.cwl_data.supports_id85 = supported,
                87 => st.cwl_data.supports_id87 = supported,
                88 => st.cwl_data.supports_id88 = supported,
                _ => {}
            }
            info!("  ID {} ({}): {}", id, name, if supported { "supported" } else { "not supported" });

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        self.probe_complete = true;
        info!("OpenTherm: Probe complete");
    }

    #[cfg(feature = "simulate-ot")]
    fn simulate(&mut self, now_ms: u32) {
        let t = now_ms as f32 / 1000.0;
        let mut st = self.state.lock().unwrap();

        st.cwl_data.connected = true;
        st.cwl_data.last_response_ms = now_ms;

        // Oscillating temperatures
        st.cwl_data.supply_inlet_temp = 18.0 + 5.0 * (t / 60.0).sin();
        st.cwl_data.exhaust_inlet_temp = 21.0 + 2.0 * (t / 45.0).sin();
        st.cwl_data.supply_outlet_temp = 20.0 + 3.0 * (t / 50.0).sin();
        st.cwl_data.exhaust_outlet_temp = 16.0 + 4.0 * (t / 55.0).sin();

        // Ventilation tracks requested level
        st.cwl_data.ventilation_level = st.requested_vent_level;
        let vent_map = [0u8, 51, 67, 100];
        let idx = (st.requested_vent_level as usize).min(3);
        st.cwl_data.relative_ventilation = vent_map[idx];

        // Status
        st.cwl_data.fault = false;
        st.cwl_data.ventilation_active = st.requested_bypass_open;
        st.cwl_data.filter_dirty = false;

        // Simulated support flags
        st.cwl_data.supports_id81 = true;
        st.cwl_data.supports_id83 = true;

        // Simulated TSP values
        st.cwl_data.tsp_values[52] = 130;
        st.cwl_data.tsp_values[53] = 0;
        st.cwl_data.tsp_valid[52] = true;
        st.cwl_data.tsp_valid[53] = true;
        st.cwl_data.tsp_values[55] = 114; // 14°C
        st.cwl_data.tsp_valid[55] = true;
        st.cwl_data.tsp_values[56] = 121; // 21°C
        st.cwl_data.tsp_valid[56] = true;
        st.cwl_data.update_derived_values();
    }
}
