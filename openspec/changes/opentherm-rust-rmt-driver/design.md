## Context

Current driver call chain:
```
ot_master.rs
  → opentherm_ffi.rs  (Rust safe wrapper)
    → opentherm_ffi.cpp  (C linkage glue)
      → OpenTherm.cpp  (protocol + noInterrupts() TX)
        → Arduino.h shim  (ESP-IDF ↔ Arduino ABI)
```

`OpenTherm.cpp::sendRequestAync()` calls `noInterrupts()` before the first bit and `interrupts()` after the last bit — a critical section that spans the full 34-bit frame (~34 ms). The WiFi driver depends on level-2 hardware interrupts during this window; losing them for 34 ms causes TCP retransmits that manifest as lag during OTA upload.

The new module collapses this to a single Rust file with no C dependency:
```
ot_master.rs
  → opentherm.rs  (RMT driver, pure Rust)
    → esp-idf-sys / esp-idf-hal  (already a dependency)
```

## Goals / Non-Goals

**Goals:**
- Eliminate the 34 ms interrupt-disable window during TX.
- Keep `ot_master.rs` API-compatible (same types, same method signatures).
- Remove all C++ build machinery.
- Retain `simulate-ot` for local development.

**Non-Goals:**
- Slave mode (the device is always master).
- Changing any polling logic in `ot_master.rs`.
- Supporting ESP32-S2/S3/C3 — the existing firmware targets ESP32 only.

## OpenTherm Protocol Recap

**Frame format** (34 bits, MSB first):
```
[31] parity (odd over bits 30-0)
[30:28] message type (3 bits)
[27:24] spare (4 bits, always 0)
[23:16] data ID (8 bits)
[15:0]  data value (16 bits)
```

**Bit encoding** (Manchester-like, 1 ms per bit, 500 µs per half-bit):
```
bit 1: wire LOW 500 µs → HIGH 500 µs
bit 0: wire HIGH 500 µs → LOW 500 µs
idle:  wire HIGH
```

**Frame structure:** start bit (always 1) + 32 data bits MSB-first + stop bit (always 1).  
**Round-trip timing:** TX ~34 ms, slave response within ~20–100 ms, master timeout 1 s.

## RMT Peripheral Overview

The ESP32 RMT (Remote Control) peripheral encodes/decodes arbitrary pulse sequences using a dedicated hardware FIFO. It runs from the APB clock (80 MHz), programmable divider → arbitrary tick duration. TX sends a pre-loaded buffer of `(duration, level)` pairs; RX captures edges with hardware-timestamped `(duration, level)` pairs. Neither TX nor RX requires interrupt disabling.

**Clock configuration:**  
`RMT_CLK_SRC_APB` (80 MHz) ÷ 80 = 1 µs/tick. Duration values are therefore directly in µs.

**RMT symbol** (`rmt_symbol_word_t`): two half-symbols packed into one u32:
```
[31:16] second half: duration1 (15 bits) + level1 (1 bit)
[15:0]  first half:  duration0 (15 bits) + level0 (1 bit)
```

## Decisions

### 1. Use `esp-idf-sys` raw bindings for RMT, not `esp-idf-hal`

**Choice:** Call `rmt_new_tx_channel`, `rmt_new_rx_channel`, `rmt_transmit`, `rmt_receive` etc. via `esp_idf_sys::*` bindings directly.

**Rationale:** `esp-idf-hal`'s RMT wrapper is still evolving and its safe API surface changes between minor versions. The raw C API is stable, well-documented, and already available through `esp-idf-sys` which is pulled in by `esp-idf-svc`. Using it keeps the dependency footprint unchanged and makes the implementation straightforward to follow against the ESP-IDF docs.

### 2. Blocking `send_request` using a FreeRTOS event group (or semaphore)

**Choice:** `send_request` is synchronous (matches current behaviour). Internally:
1. Load TX buffer and call `rmt_transmit()` (non-blocking in RMT driver, completes via DMA).
2. Call `rmt_receive()` to arm RX capture with 1100 ms timeout.
3. Block on `rmt_rx_done_callback` via a FreeRTOS semaphore/event group.
4. Decode captured symbols in place.

**Rationale:** `ot_master.rs` already runs on its own thread; blocking there is fine and keeps the interface identical to the current C++ driver. A FreeRTOS semaphore gives the thread back to the scheduler while waiting instead of busy-spinning.

**Alternative considered:** Non-blocking with a completion callback. Rejected — would require `ot_master.rs` restructuring.

### 3. RMT TX encoding

Each bit is encoded as one `rmt_symbol_word_t`:
```
bit 1: { duration0: 500, level0: 0, duration1: 500, level1: 1 }  // LOW 500µs → HIGH 500µs
bit 0: { duration0: 500, level0: 1, duration1: 500, level1: 0 }  // HIGH 500µs → LOW 500µs
```
Frame = 34 symbols (start + 32 data + stop), followed by the RMT end-of-transmission marker `{ 0, 0, 0, 0 }`.

TX idle level configured to HIGH (passive bus state).

### 4. RMT RX decoding via half-bit expansion

**Choice:** Expand captured RMT symbols into a flat half-bit list, then decode pairs.

**Algorithm:**
1. For each captured symbol `(d0, l0, d1, l1)` (filtering zero-duration end markers):
   - Push `l0` once if `d0 ≤ 750 µs`, twice if `d0 > 750 µs`.
   - Push `l1` once if `d1 ≤ 750 µs`, twice if `d1 > 750 µs`.
2. Require exactly 68 half-bits (34 bits × 2).
3. Decode pairs:
   - `(LOW, HIGH)` → bit 1
   - `(HIGH, LOW)` → bit 0
   - any other pair → `ResponseStatus::Invalid`
4. Skip pair 0 (start) and pair 33 (stop). Reconstruct 32-bit frame from pairs 1–32.
5. Verify odd parity over all 32 bits. Fail → `ResponseStatus::Invalid`.

**Why this works:** When two consecutive same-level half-bits merge into a ~1000 µs pulse, expanding it as two copies of the same level naturally feeds one into the tail of the current bit-pair and one into the head of the next, without needing explicit boundary tracking. The correctness proof holds for all 34 valid OpenTherm bit combinations.

### 5. RMT RX filter and minimum pulse width

Configure `rmt_receive_config_t`:
- `signal_range_min_ns`: 1000 ns (1 µs) — filters sub-µs glitches.
- `signal_range_max_ns`: 1_500_000 ns (1500 µs) — caps merge window; pulses longer than 1500 µs indicate bus fault.

### 6. Recovery delay

After each exchange (success, invalid, or timeout), `send_request` blocks for 100 ms before returning. This matches the C++ `DELAY` state and prevents back-to-back requests from violating the OpenTherm inter-message gap.

### 7. Module structure

```
src/opentherm.rs
  pub enum MessageType        // ReadData, WriteData, ReadAck, WriteAck, ...
  pub enum ResponseStatus     // None, Success, Invalid, Timeout
  pub struct OpenTherm        // the driver
    pub fn new(in_pin, out_pin) -> Result<Self, EspError>
    pub fn send_request(&self, request: u32) -> u32
    pub fn last_status(&self) -> ResponseStatus
    pub fn build_request(msg_type, data_id, data) -> u32   // pure
    pub fn get_message_type(frame: u32) -> MessageType     // pure
  // private:
  fn encode_frame(frame: u32) -> [rmt_symbol_word_t; 35]  // 34 + end marker
  fn decode_symbols(items: &[rmt_symbol_word_t]) -> Result<u32, ()>
  fn odd_parity(frame: u32) -> bool
```

Under `#[cfg(feature = "simulate-ot")]` the struct body is replaced with mock counters (mirrors the current `opentherm_ffi.rs` simulate path in `ot_master.rs`).

## Risks / Trade-offs

- **RMT RX jitter:** The hardware timestamping is accurate to ~1 µs. With a 750 µs threshold and ±50 µs real-world tolerance the half-bit expansion is reliable. A noisy bus could produce pulses near the threshold; the `signal_range_min_ns` filter mitigates transient spikes.
- **RMT channel availability:** ESP32 has 8 RMT channels (4 TX, 4 RX in v5 driver). One TX and one RX channel are consumed. Other peripherals (IR, LED strip) on the same board compete for these; this project uses no other RMT consumers.
- **`esp-idf-sys` raw binding breakage:** If ESP-IDF is upgraded past a major version the raw RMT API may change (it did between v4 and v5). The risk is accepted — the firmware is pinned to a specific ESP-IDF version via `.embuild`.
