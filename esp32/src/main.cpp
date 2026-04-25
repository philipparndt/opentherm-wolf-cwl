/**
 * Wolf CWL OpenTherm Controller
 *
 * Controls a Wolf CWL 300 ventilation unit via OpenTherm protocol
 * using a DIYLess Master OpenTherm Shield, with OLED display and
 * rotary encoder for local control.
 *
 * Features:
 * - OpenTherm master: ventilation level, bypass, temperatures, TSP
 * - MQTT integration for Home Assistant
 * - SH1106 OLED display with rotary encoder navigation
 * - Web UI for configuration
 * - OTA firmware updates
 */

#include <Arduino.h>
#include <esp_random.h>
#include <time.h>

#include "config.h"
#include "config_manager.h"
#include "logging.h"
#include "network.h"
#include "ot_master.h"
#include "wolf_cwl.h"
#include "mqtt_client.h"
#include "display.h"
#include "encoder.h"
#include "status.h"
#include "webserver.h"
#include "ap_mode.h"
#include "watchdog.h"
#include "scheduler.h"

// =============================================================================
// Timing constants
// =============================================================================
#define STATUS_REPORT_INTERVAL  60000
#define DISPLAY_UPDATE_INTERVAL 500
#define FACTORY_RESET_HOLD_MS   10000

// =============================================================================
// State
// =============================================================================
static unsigned long lastStatusReport = 0;
static unsigned long lastDisplayUpdate = 0;
static bool bridgeInfoPublished = false;

// =============================================================================
// Factory reset check (hold encoder button during boot)
// =============================================================================
static void checkFactoryReset() {
    pinMode(appConfig.encSwPin, INPUT_PULLUP);
    delay(100);

    if (digitalRead(appConfig.encSwPin) == LOW) {
        Serial.println("Encoder button held at boot - hold for 10s to factory reset...");
        unsigned long start = millis();

        while (digitalRead(appConfig.encSwPin) == LOW) {
            if (millis() - start > FACTORY_RESET_HOLD_MS) {
                Serial.println("FACTORY RESET!");
                resetConfig();
                delay(1000);
                ESP.restart();
            }
            delay(100);
        }
        Serial.println("Button released, continuing normal boot");
    }
}

// =============================================================================
// Setup
// =============================================================================
void setup() {
    Serial.begin(115200);
    delay(1000);

    Serial.println("\n\nWolf CWL OpenTherm Controller");
    Serial.println("==============================");

    // Load configuration from NVS (or migrate from config.h)
    loadConfig();

    // Check for factory reset (hold encoder button at boot)
    checkFactoryReset();

    // Initialize data model
    initCwlData();

    // Initialize components
    setupStatusLed();
    setupDisplay();
    setupEncoder();
    randomSeed(esp_random());

    // Check if we should start in AP mode (unconfigured)
    if (shouldStartApMode()) {
        log("Starting in AP mode for initial configuration...");
        setupApMode();
        setupWebServer();
        log("Setup complete - waiting for configuration via web UI");
        return;
    }

    // Normal operation mode
    setupNetwork();

    // Configure NTP with CET/CEST timezone
    configTzTime("CET-1CEST,M3.5.0,M10.5.0/3", "pool.ntp.org", "time.nist.gov");
    Serial.println("Waiting for NTP time sync...");
    time_t now_time = time(nullptr);
    int attempts = 0;
    while (now_time < 1700000000 && attempts < 20) {
        delay(500);
        now_time = time(nullptr);
        attempts++;
    }
    if (now_time > 1700000000) {
        struct tm timeinfo;
        localtime_r(&now_time, &timeinfo);
        log("NTP time synced: " + String(timeinfo.tm_hour) + ":" + String(timeinfo.tm_min));
    }

    // Initialize MQTT
    setupMqtt();

    // Initialize OpenTherm
    setupOpenTherm();

    // Initialize scheduler
    setupScheduler();

    // Initialize watchdog (after all subsystems are up)
    setupWatchdog();

    // Initialize Web Server
    setupWebServer();

    log("Setup complete");
    printSystemStatus();
}

// =============================================================================
// Main Loop
// =============================================================================
void loop() {
    unsigned long now = millis();

    // Feed hardware watchdog + run software liveness checks
    feedWatchdog();

    // AP mode handling
    if (apModeActive) {
        apModeLoop();
        webServerLoop();
        encoderLoop();
        if (now - lastDisplayUpdate > DISPLAY_UPDATE_INTERVAL) {
            lastDisplayUpdate = now;
            updateDisplay();
        }
        delay(10);
        return;
    }

    // Network handling
    networkLoop();

    // Web server handling
    webServerLoop();

    // Encoder handling
    encoderLoop();

    // LED status
    updateStatusLed();

    if (!networkConnected) {
        // Still run OpenTherm and display even without network
        openThermLoop();
        if (now - lastDisplayUpdate > DISPLAY_UPDATE_INTERVAL) {
            lastDisplayUpdate = now;
            updateDisplay();
        }
        delay(10);
        return;
    }

    // MQTT handling
    if (!mqtt.connected()) {
        mqttReconnect();
    }
    mqttLoop();

    // Publish bridge info once after MQTT connects
    if (mqtt.connected() && !bridgeInfoPublished) {
        bridgeInfoPublished = true;
        publishBridgeInfo();
    }
    if (!mqtt.connected()) {
        bridgeInfoPublished = false;
    }

    // Schedule evaluation (ventilation + bypass)
    schedulerLoop();

    // OpenTherm polling cycle
    openThermLoop();

    // Display update (throttled)
    if (now - lastDisplayUpdate > DISPLAY_UPDATE_INTERVAL) {
        lastDisplayUpdate = now;
        updateDisplay();
    }

    // Periodic status report
    if (now - lastStatusReport > STATUS_REPORT_INTERVAL) {
        lastStatusReport = now;
        printSystemStatus();
    }

    delay(10);
}
