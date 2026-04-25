#include "display.h"
#include "config_manager.h"
#include "wolf_cwl.h"
#include "ot_master.h"
#include "network.h"
#include "mqtt_client.h"
#include "logging.h"
#include "watchdog.h"
#include "scheduler.h"
#include <U8g2lib.h>
#include <Wire.h>
#include <WiFi.h>

// SH1106 128x64 I2C display
static U8G2_SH1106_128X64_NONAME_F_HW_I2C u8g2(U8G2_R0, U8X8_PIN_NONE);

int currentPage = PAGE_HOME;
bool editMode = false;
static uint8_t editVentLevel = 0;
static unsigned long editModeStart = 0;
#define EDIT_TIMEOUT 10000  // 10 seconds

static bool displayInitialized = false;

void setupDisplay() {
    Wire.begin(appConfig.sdaPin, appConfig.sclPin);
    u8g2.begin();
    u8g2.setContrast(200);
    displayInitialized = true;

    // Boot screen
    u8g2.clearBuffer();
    u8g2.setFont(u8g2_font_helvB14_tr);
    u8g2.drawStr(20, 28, "Wolf CWL");
    u8g2.setFont(u8g2_font_helvR08_tr);
    u8g2.drawStr(20, 48, "Connecting...");
    u8g2.sendBuffer();

    log("Display: SH1106 initialized (SDA=" + String(appConfig.sdaPin) +
        ", SCL=" + String(appConfig.sclPin) + ")");
}

// =============================================================================
// Page renderers
// =============================================================================

static void drawHeader(const char* title) {
    u8g2.setFont(u8g2_font_helvR08_tr);
    u8g2.drawStr(0, 8, title);
    u8g2.drawHLine(0, 10, 128);
}

static void drawHomePage() {
    // Large ventilation level
    if (editMode) {
        drawHeader("Set Level");
        u8g2.setFont(u8g2_font_helvB24_tr);
        const char* name = getVentilationLevelName(editVentLevel);
        int w = u8g2.getStrWidth(name);
        u8g2.drawStr((128 - w) / 2, 42, name);

        // Edit indicator
        u8g2.setFont(u8g2_font_helvR08_tr);
        u8g2.drawStr(0, 62, "\x4 rotate  \x5 confirm");
    } else {
        drawHeader("Ventilation");
        u8g2.setFont(u8g2_font_helvB24_tr);
        const char* name = getVentilationLevelName(cwlData.ventilationLevel);
        int w = u8g2.getStrWidth(name);
        u8g2.drawStr((128 - w) / 2, 40, name);

        // Status line with schedule indicator
        u8g2.setFont(u8g2_font_helvR08_tr);
        char buf[40];
        const char* indicator = scheduleOverride ? " M" : (scheduleActive ? " S" : "");
        snprintf(buf, sizeof(buf), "%d%%%s  %s",
                 cwlData.relativeVentilation, indicator,
                 requestedBypassOpen ? "Summer" : "Winter");
        u8g2.drawStr(0, 58, buf);
    }
}

static void drawTemperaturePage() {
    drawHeader("Temperatures");
    u8g2.setFont(u8g2_font_helvR10_tr);

    char buf[32];
    snprintf(buf, sizeof(buf), "Supply in:  %.1f C", cwlData.supplyInletTemp);
    u8g2.drawStr(0, 26, buf);

    snprintf(buf, sizeof(buf), "Exhaust in: %.1f C", cwlData.exhaustInletTemp);
    u8g2.drawStr(0, 40, buf);

    if (cwlData.supportsId81) {
        snprintf(buf, sizeof(buf), "Supply out: %.1f C", cwlData.supplyOutletTemp);
        u8g2.drawStr(0, 54, buf);
    }

    if (cwlData.tspValid[55]) {
        snprintf(buf, sizeof(buf), "Outdoor:    %d C", cwlData.tempAtmospheric);
        u8g2.drawStr(0, cwlData.supportsId81 ? 64 : 54, buf);
    }
}

static void drawStatusPage() {
    drawHeader("Status");
    u8g2.setFont(u8g2_font_helvR08_tr);

    char buf[32];
    int y = 22;

    snprintf(buf, sizeof(buf), "Fault:   %s", cwlData.fault ? "YES" : "No");
    u8g2.drawStr(0, y, buf); y += 11;

    snprintf(buf, sizeof(buf), "Filter:  %s", cwlData.filterDirty ? "REPLACE" : "OK");
    u8g2.drawStr(0, y, buf); y += 11;

    snprintf(buf, sizeof(buf), "Mode:    %s", requestedBypassOpen ? "Summer" : "Winter");
    u8g2.drawStr(0, y, buf); y += 11;

    if (cwlData.tspValid[52]) {
        snprintf(buf, sizeof(buf), "Airflow: %d m3/h", cwlData.currentVolume);
        u8g2.drawStr(0, y, buf);
    }
}

static void drawSystemPage() {
    drawHeader("System");
    u8g2.setFont(u8g2_font_helvR08_tr);

    char buf[32];
    int y = 22;

    if (networkConnected) {
        snprintf(buf, sizeof(buf), "WiFi: %ddBm", WiFi.RSSI());
        u8g2.drawStr(0, y, buf); y += 11;

        String ip = WiFi.localIP().toString();
        u8g2.drawStr(0, y, ip.c_str()); y += 11;
    } else {
        u8g2.drawStr(0, y, "WiFi: disconnected"); y += 11;
        y += 11;
    }

    snprintf(buf, sizeof(buf), "MQTT: %s", mqtt.connected() ? "connected" : "offline");
    u8g2.drawStr(0, y, buf); y += 11;

    unsigned long upSec = getUptimeSeconds();
    unsigned long upMin = upSec / 60;
    unsigned long upHour = upMin / 60;
    snprintf(buf, sizeof(buf), "Up: %luh %lum  %s #%lu", upHour, upMin % 60,
             getRebootReasonStr(), (unsigned long)getCrashCount());
    u8g2.drawStr(0, y, buf);
}

// =============================================================================
// Public functions
// =============================================================================

void updateDisplay() {
    if (!displayInitialized) return;

    // Edit mode timeout
    if (editMode && millis() - editModeStart > EDIT_TIMEOUT) {
        exitEditMode(false);
    }

    u8g2.clearBuffer();

    switch (currentPage) {
        case PAGE_HOME:        drawHomePage(); break;
        case PAGE_TEMPERATURE: drawTemperaturePage(); break;
        case PAGE_STATUS:      drawStatusPage(); break;
        case PAGE_SYSTEM:      drawSystemPage(); break;
    }

    // Page indicator dots
    u8g2.setFont(u8g2_font_helvR08_tr);
    for (int i = 0; i < PAGE_COUNT; i++) {
        int x = 56 + i * 6;
        if (i == currentPage) {
            u8g2.drawDisc(x, 63, 2);
        } else {
            u8g2.drawCircle(x, 63, 2);
        }
    }

    u8g2.sendBuffer();
}

void setPage(int page) {
    if (page >= 0 && page < PAGE_COUNT) {
        currentPage = page;
    }
}

void nextPage() {
    currentPage = (currentPage + 1) % PAGE_COUNT;
}

void prevPage() {
    currentPage = (currentPage - 1 + PAGE_COUNT) % PAGE_COUNT;
}

void enterEditMode() {
    if (currentPage == PAGE_HOME) {
        editMode = true;
        editVentLevel = cwlData.ventilationLevel;
        editModeStart = millis();
    }
}

void exitEditMode(bool apply) {
    if (apply) {
        requestedVentLevel = editVentLevel;
        appConfig.ventilationLevel = editVentLevel;
        saveConfig();
        setVentilationManualOverride();
        log("Display: Ventilation set to " + String(editVentLevel) +
            " (" + getVentilationLevelName(editVentLevel) + ")");
    }
    editMode = false;
}

void adjustEditValue(int delta) {
    if (!editMode) return;
    editModeStart = millis();  // Reset timeout

    int newLevel = (int)editVentLevel + delta;
    if (newLevel < 0) newLevel = 3;
    if (newLevel > 3) newLevel = 0;
    editVentLevel = newLevel;
}
