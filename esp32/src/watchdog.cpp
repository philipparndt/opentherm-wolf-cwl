#include "watchdog.h"
#include "wolf_cwl.h"
#include "logging.h"
#include <esp_task_wdt.h>
#include <esp_system.h>
#include <Preferences.h>

// =============================================================================
// Configuration
// =============================================================================
#define WDT_TIMEOUT_S        30       // Hardware watchdog: 30 seconds
#define OT_LIVENESS_TIMEOUT  300000   // OpenTherm: restart after 5 minutes no response
#define OT_STARTUP_GRACE     60000    // Wait 60s after boot before checking OT liveness
#define HEAP_MIN_BYTES       20480    // Restart if free heap drops below 20 KB
#define LIVENESS_CHECK_INTERVAL 10000 // Run soft checks every 10 seconds

// =============================================================================
// State
// =============================================================================
static uint32_t crashCount = 0;
static esp_reset_reason_t resetReason;
static unsigned long lastLivenessCheck = 0;
static unsigned long bootTime = 0;

// =============================================================================
// Reboot reason decoder
// =============================================================================
const char* getRebootReasonStr() {
    switch (resetReason) {
        case ESP_RST_POWERON:  return "Power";
        case ESP_RST_EXT:      return "External";
        case ESP_RST_SW:       return "SW";
        case ESP_RST_PANIC:    return "Panic";
        case ESP_RST_INT_WDT:  return "IntWDT";
        case ESP_RST_TASK_WDT: return "TaskWDT";
        case ESP_RST_WDT:      return "WDT";
        case ESP_RST_DEEPSLEEP:return "DeepSleep";
        case ESP_RST_BROWNOUT: return "Brownout";
        case ESP_RST_SDIO:     return "SDIO";
        default:               return "Unknown";
    }
}

static bool isAbnormalReset(esp_reset_reason_t reason) {
    return reason == ESP_RST_PANIC ||
           reason == ESP_RST_INT_WDT ||
           reason == ESP_RST_TASK_WDT ||
           reason == ESP_RST_WDT ||
           reason == ESP_RST_BROWNOUT;
}

// =============================================================================
// Setup
// =============================================================================
void setupWatchdog() {
    bootTime = millis();
    resetReason = esp_reset_reason();

    // Load crash counter from NVS
    Preferences prefs;
    prefs.begin("watchdog", true);
    crashCount = prefs.getUInt("crash_count", 0);
    prefs.end();

    // Increment crash counter on abnormal reset
    if (isAbnormalReset(resetReason)) {
        crashCount++;
        Preferences prefsW;
        prefsW.begin("watchdog", false);
        prefsW.putUInt("crash_count", crashCount);
        prefsW.end();
        log("Watchdog: Abnormal reset detected (" + String(getRebootReasonStr()) +
            "), crash count: " + String(crashCount));
    } else {
        log("Watchdog: Boot reason: " + String(getRebootReasonStr()) +
            ", crash count: " + String(crashCount));
    }

    // Initialize hardware Task Watchdog Timer
    esp_task_wdt_init(WDT_TIMEOUT_S, true);  // true = trigger reset on timeout
    esp_task_wdt_add(NULL);  // Add current task (loopTask)

    log("Watchdog: TWDT enabled (" + String(WDT_TIMEOUT_S) + "s timeout)");
}

// =============================================================================
// Feed watchdog + software liveness checks
// =============================================================================
void feedWatchdog() {
    // Feed hardware watchdog
    esp_task_wdt_reset();

    // Run soft liveness checks periodically
    unsigned long now = millis();
    if (now - lastLivenessCheck < LIVENESS_CHECK_INTERVAL) return;
    lastLivenessCheck = now;

    // OpenTherm liveness check (with startup grace period)
    if (now - bootTime > OT_STARTUP_GRACE) {
        if (cwlData.lastResponse > 0) {
            unsigned long otAge = now - cwlData.lastResponse;
            if (otAge > OT_LIVENESS_TIMEOUT) {
                log("Watchdog: OpenTherm unresponsive for " + String(otAge / 1000) +
                    "s, restarting...");
                delay(100);
                ESP.restart();
            }
        } else {
            // Never received a response — check if we've been waiting too long
            unsigned long waitTime = now - bootTime;
            if (waitTime > OT_LIVENESS_TIMEOUT) {
                log("Watchdog: No OpenTherm response since boot (" +
                    String(waitTime / 1000) + "s), restarting...");
                delay(100);
                ESP.restart();
            }
        }
    }

    // Heap exhaustion check
    uint32_t freeHeap = ESP.getFreeHeap();
    if (freeHeap < HEAP_MIN_BYTES) {
        log("Watchdog: Heap critically low (" + String(freeHeap) +
            " bytes), restarting...");
        delay(100);
        ESP.restart();
    }
}

// =============================================================================
// Diagnostics getters
// =============================================================================
uint32_t getCrashCount() {
    return crashCount;
}

unsigned long getUptimeSeconds() {
    return millis() / 1000;
}

unsigned long getOtResponseAge() {
    if (cwlData.lastResponse == 0) return millis() / 1000;  // Never received
    return (millis() - cwlData.lastResponse) / 1000;
}
