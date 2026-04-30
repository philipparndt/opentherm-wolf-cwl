//! Watchdog — task WDT, liveness checks, crash counter.

use esp_idf_svc::sys::*;
use log::{info, warn};

const WDT_TIMEOUT_S: u32 = 30;
const LIVENESS_INTERVAL_MS: u32 = 10_000;
const OT_TIMEOUT_MS: u32 = 300_000; // 5 minutes
const MIN_FREE_HEAP: u32 = 20_480;  // 20 KB

pub struct Watchdog {
    last_check_ms: u32,
    startup_grace_ms: u32,
}

impl Watchdog {
    pub fn new() -> Self {
        unsafe {
            let config = esp_task_wdt_config_t {
                timeout_ms: WDT_TIMEOUT_S * 1000,
                idle_core_mask: 0,
                trigger_panic: true,
            };
            // ESP-IDF already initializes TWDT — just reconfigure with our timeout
            esp_task_wdt_reconfigure(&config);
            // Add current task to WDT
            let handle = xTaskGetCurrentTaskHandle();
            let ret = esp_task_wdt_add(handle);
            if ret != ESP_OK {
                // Task might already be added — delete and re-add
                esp_task_wdt_delete(handle);
                esp_task_wdt_add(handle);
            }
        }
        info!("Watchdog: TWDT enabled ({}s timeout)", WDT_TIMEOUT_S);

        Self {
            last_check_ms: 0,
            startup_grace_ms: 60_000,
        }
    }

    /// Feed the watchdog and run liveness checks. Call from main loop.
    pub fn update(&mut self, now_ms: u32, ot_last_response_ms: u32, ot_connected: bool) {
        // Feed the watchdog
        unsafe { esp_task_wdt_reset() };

        if now_ms < self.startup_grace_ms {
            return; // Grace period after boot
        }

        if now_ms.wrapping_sub(self.last_check_ms) < LIVENESS_INTERVAL_MS {
            return;
        }
        self.last_check_ms = now_ms;

        // Check OpenTherm liveness
        if ot_connected && now_ms.wrapping_sub(ot_last_response_ms) > OT_TIMEOUT_MS {
            warn!("Watchdog: OpenTherm not responding for 5 min, restarting");
            unsafe { esp_restart() };
        }

        // Check heap
        let free_heap = unsafe { esp_get_free_heap_size() };
        if free_heap < MIN_FREE_HEAP {
            warn!("Watchdog: Low heap ({}), restarting", free_heap);
            unsafe { esp_restart() };
        }
    }
}

/// Get reboot reason as string
pub fn reboot_reason() -> &'static str {
    #[allow(non_upper_case_globals)]
    match unsafe { esp_reset_reason() } {
        esp_reset_reason_t_ESP_RST_POWERON => "Power",
        esp_reset_reason_t_ESP_RST_SW => "SW",
        esp_reset_reason_t_ESP_RST_PANIC => "Panic",
        esp_reset_reason_t_ESP_RST_INT_WDT => "IntWDT",
        esp_reset_reason_t_ESP_RST_TASK_WDT => "TaskWDT",
        esp_reset_reason_t_ESP_RST_WDT => "WDT",
        esp_reset_reason_t_ESP_RST_BROWNOUT => "Brownout",
        _ => "Unknown",
    }
}

/// Get uptime in seconds
#[allow(dead_code)]
pub fn uptime_seconds() -> u64 {
    unsafe { esp_timer_get_time() as u64 / 1_000_000 }
}
