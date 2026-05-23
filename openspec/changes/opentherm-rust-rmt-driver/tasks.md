## 1. New Rust driver — pure types and frame helpers

- [x] 1.1 Create `src/opentherm.rs` with `MessageType` and `ResponseStatus` enums (copy from `opentherm_ffi.rs`, keep identical variants and `#[repr(u8)]`)
- [x] 1.2 Add `build_request(msg_type: MessageType, data_id: u8, data: u16) -> u32` — pure function: place data at bits 15-0, data_id at bits 23-16, set bit 28 if WriteData, compute odd parity and set bit 31
- [x] 1.3 Add `get_message_type(frame: u32) -> MessageType` — extract bits 30-28
- [x] 1.4 Add `odd_parity(frame: u32) -> bool` helper (count 1-bits, return true if odd)

## 2. RMT TX — frame encoding

- [x] 2.1 Add `encode_frame(frame: u32) -> [rmt_symbol_word_t; 35]` — 34 symbols (start + 32 data MSB-first + stop) plus one zero end-marker; bit 1 = `{500, LOW, 500, HIGH}`, bit 0 = `{500, HIGH, 500, LOW}`
- [x] 2.2 Initialize RMT TX channel in `OpenTherm::new`: `rmt_new_tx_channel` with `gpio_num = out_pin`, `clk_src = RMT_CLK_SRC_APB`, `resolution_hz = 1_000_000` (1 µs/tick), idle level HIGH, mem_block_symbols ≥ 35
- [x] 2.3 Create `rmt_copy_encoder` for TX (no modulation needed — raw symbols only)
- [x] 2.4 Enable the TX channel with `rmt_enable`

## 3. RMT RX — frame capture

- [x] 3.1 Initialize RMT RX channel in `OpenTherm::new`: `rmt_new_rx_channel` with `gpio_num = in_pin`, `clk_src = RMT_CLK_SRC_APB`, `resolution_hz = 1_000_000`, `signal_range_min_ns = 1_000`, `signal_range_max_ns = 1_500_000`, mem_block_symbols ≥ 128
- [x] 3.2 Register `rmt_rx_event_callbacks` with an `on_recv_done` callback that gives a FreeRTOS semaphore (allocated in `OpenTherm::new`)
- [x] 3.3 Enable the RX channel with `rmt_enable`
- [x] 3.4 Allocate a receive buffer `[rmt_symbol_word_t; 128]` owned by the `OpenTherm` struct

## 4. Half-bit decoder

- [x] 4.1 Add `decode_symbols(items: &[rmt_symbol_word_t]) -> Result<u32, ()>` following the half-bit expansion algorithm from the design:
  - Expand each symbol's two half-slots into the `half_bits` vec, doubling a slot if its duration > 750
  - Return `Err(())` if `half_bits.len() != 68`
  - Decode 34 pairs: `(LOW, HIGH)` → 1, `(HIGH, LOW)` → 0, else `Err(())`
  - Skip pair 0 (start bit) and pair 33 (stop bit)
  - Reconstruct u32 from pairs 1–32
  - Return `Err(())` if `odd_parity(result)` is false (parity bit included)

## 5. Blocking send_request

- [x] 5.1 Implement `send_request(&self, request: u32) -> u32`:
  1. Call `encode_frame(request)` to get TX symbols
  2. Call `rmt_receive` to arm RX (do this BEFORE transmitting so no leading edge is missed)
  3. Call `rmt_transmit` with the encoded symbols
  4. Block on the FreeRTOS semaphore with `xSemaphoreTake(sem, 1100 ms ticks)`
  5. If semaphore times out: set `last_status = ResponseStatus::Timeout`, sleep 100 ms, return 0
  6. Call `decode_symbols` on the receive buffer
  7. On `Err`: set `last_status = ResponseStatus::Invalid`, sleep 100 ms, return 0
  8. On `Ok(frame)`: set `last_status = ResponseStatus::Success`, sleep 100 ms, return frame
- [x] 5.2 Store `last_status` in the struct (use `Cell<ResponseStatus>` or a Mutex since the struct is used from a single thread)
- [x] 5.3 Implement `last_status(&self) -> ResponseStatus` to return the stored value

## 6. Simulate-OT feature

- [x] 6.1 Gate the entire `OpenTherm` struct body (RMT fields) behind `#[cfg(not(feature = "simulate-ot"))]`
- [x] 6.2 Add a `#[cfg(feature = "simulate-ot")]` `OpenTherm` stub that keeps `cargo check --features simulate-ot` clean. (Scope note: moving the full simulation logic into `opentherm.rs` would be a larger refactor — kept the existing `ot_master::simulate()` path as-is; the stub exists only so the import resolves.)

## 7. Wire up the new module

- [x] 7.1 In `src/main.rs` (or wherever `opentherm_ffi` is declared as a module): replace `mod opentherm_ffi;` with `mod opentherm;`
- [x] 7.2 In `src/ot_master.rs`: change `use crate::opentherm_ffi::` imports to `use crate::opentherm::`
- [x] 7.3 Verify that `OpenTherm::new`, `send_request`, `last_status`, `build_request`, `get_message_type` all resolve correctly

## 8. Remove C++ component and build machinery

- [x] 8.1 Delete `src/opentherm_ffi.rs`
- [x] 8.2 Delete `components/opentherm/` directory (OpenTherm.cpp, OpenTherm.h, opentherm_ffi.cpp, opentherm_ffi.h, Arduino.h, FunctionalInterrupt.h, CMakeLists.txt if present)
- [x] 8.3 In `build.rs`: delete the `compile_opentherm()` function and its call site; keep the WiFi credential forwarding section unchanged
- [x] 8.4 In `Cargo.toml`: remove `cc = "1"` from `[build-dependencies]`
- [x] 8.5 In `Cargo.toml` `[package.metadata.esp-idf-sys]`: confirm `extra_components = []` (the C++ component was not listed there anyway, but verify)

## 9. Verify

- [x] 9.1 `cargo check` passes (no feature flags) — confirms the new module compiles and types align
- [x] 9.2 `cargo check --features simulate-ot` passes — confirms simulate path still works
- [ ] 9.3 `cargo build --release` produces a firmware binary without linker errors *(not run — slow xtensa link; verify on real device)*
- [ ] 9.4 Flash firmware and confirm: OTA upload completes without visible lag, OpenTherm polling resumes normally after reboot, sensor data appears on MQTT within 30 s of boot *(requires hardware)*
