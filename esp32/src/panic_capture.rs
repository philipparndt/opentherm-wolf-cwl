//! Capture the last Rust panic (message + `file:line`) across the reboot it
//! triggers, so a crash on an installed / USB-less unit isn't opaque.
//!
//! ESP-IDF prints the panic backtrace to UART only, and there is no coredump
//! partition, so otherwise nothing about a panic survives a reboot. A panic
//! hook writes the message + location into a `.noinit` RAM buffer — which the
//! startup code does not clear, so it survives the panic-triggered software
//! reset (as long as power is maintained). On the next boot [`load`] reads and
//! caches it for the HTTP status API, MQTT health, and the log.
//!
//! Note: an out-of-memory panic may not capture cleanly (the hook allocates via
//! `format!`); ordinary panics (`unwrap`, indexing, asserts) capture fine.

use core::ptr::{addr_of, addr_of_mut};
use std::sync::Mutex;

const BUF_LEN: usize = 240;
const MAGIC: u32 = 0x504e_4331; // "PNC1" — marks the buffer as holding a real panic

// `.noinit` is preserved across a software reset (it is not zeroed at startup),
// so these survive the panic-triggered reboot. Lost only on power-off/brownout.
#[link_section = ".noinit"]
static mut PANIC_BUF: [u8; BUF_LEN] = [0; BUF_LEN];
#[link_section = ".noinit"]
static mut PANIC_LEN: usize = 0;
#[link_section = ".noinit"]
static mut PANIC_MAGIC: u32 = 0;

// Cached copy of the panic captured on the *previous* boot (None if clean).
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// Install the panic hook. Call once, as early as possible in `main`.
pub fn install_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("panic");
        let text = format!("{loc} | {msg}");
        let bytes = text.as_bytes();
        let n = bytes.len().min(BUF_LEN);
        // Raw pointers (not references) to avoid taking refs to `static mut`.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr_of_mut!(PANIC_BUF) as *mut u8, n);
            addr_of_mut!(PANIC_LEN).write(n);
            addr_of_mut!(PANIC_MAGIC).write(MAGIC);
        }
        // Still run the default hook so the backtrace prints to UART when monitored.
        default(info);
    }));
}

/// Read + clear the panic captured before the last reboot and cache it. Call
/// once after boot. Returns the message (with `file:line`) if the previous run
/// ended in a panic, else `None`.
pub fn load() -> Option<String> {
    let captured = unsafe {
        if addr_of!(PANIC_MAGIC).read() != MAGIC {
            return None;
        }
        addr_of_mut!(PANIC_MAGIC).write(0); // consume so it's reported only once
        let n = addr_of!(PANIC_LEN).read().min(BUF_LEN);
        let slice = core::slice::from_raw_parts(addr_of!(PANIC_BUF) as *const u8, n);
        String::from_utf8_lossy(slice).into_owned()
    };
    *LAST_PANIC.lock().unwrap() = Some(captured.clone());
    Some(captured)
}

/// The panic captured before the last reboot, if any (cached by [`load`]).
pub fn last_panic() -> Option<String> {
    LAST_PANIC.lock().unwrap().clone()
}
