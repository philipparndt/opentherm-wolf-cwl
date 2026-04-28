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
#if defined(USE_ETHERNET)
  #include <ETH.h>
#endif

// 128x64 I2C OLED display — auto-detect SSD1306 vs SH1106
// We allocate both statically but only initialize the detected one
static U8G2_SSD1306_128X64_NONAME_F_HW_I2C u8g2_ssd1306(U8G2_R0, U8X8_PIN_NONE);
static U8G2_SH1106_128X64_NONAME_F_HW_I2C u8g2_sh1106(U8G2_R0, U8X8_PIN_NONE);
static U8G2* u8g2ptr = nullptr;
#define u8g2 (*u8g2ptr)

int currentPage = PAGE_HOME;
bool editMode = false;
static uint8_t editVentLevel = 0;
static bool editOffDuration = false;  // Sub-stage: selecting off duration
static uint8_t editOffHours = 1;
static unsigned long editModeStart = 0;
#define EDIT_TIMEOUT 10000  // 10 seconds

static bool displayInitialized = false;
static bool displayStandby = false;
static unsigned long lastActivityTime = 0;
#define STANDBY_TIMEOUT 300000  // 5 minutes

// Overlay state (used for connect/disconnect notifications)
static char overlayHeader[16] = {0};
static char overlayMessage[16] = {0};
static unsigned long overlayStart = 0;
static bool overlayActive = false;
#define OVERLAY_TIMEOUT 10000  // 10 seconds

// Detect display chip by probing I2C
// SSD1306 and SH1106 both respond at 0x3C, but we can differentiate
// by trying SSD1306 first — if the display shows artifacts, it's SH1106
static bool detectDisplayChip() {
    // Scan for display on I2C
    Wire.beginTransmission(0x3C);
    bool found = (Wire.endTransmission() == 0);
    if (!found) {
        Wire.beginTransmission(0x3D);
        found = (Wire.endTransmission() == 0);
    }
    if (!found) {
        log("Display: No OLED found on I2C");
        return false;
    }

    // Try SSD1306 first if configured, otherwise SH1106
    #ifdef USE_SSD1306
    u8g2ptr = &u8g2_ssd1306;
    log("Display: Using SSD1306 driver");
    #else
    u8g2ptr = &u8g2_sh1106;
    log("Display: Using SH1106 driver");
    #endif
    return true;
}

void setupDisplay() {
    lastActivityTime = millis();
    Wire.begin(appConfig.sdaPin, appConfig.sclPin);

    if (!detectDisplayChip()) {
        return;
    }

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

    log("Display: Initialized (SDA=" + String(appConfig.sdaPin) +
        ", SCL=" + String(appConfig.sclPin) + ")");
}

// =============================================================================
// Page renderers
// =============================================================================

static void drawHeader(const char* title) {
    u8g2.setFont(u8g2_font_helvR08_tr);
    u8g2.drawStr(0, 8, title);
    u8g2.drawHLine(0, 10, u8g2.getDisplayWidth());
}

static void drawLevelCentered(const char* name, int y) {
    // Try large font first, fall back to medium if too wide
    u8g2.setFont(u8g2_font_helvB18_tr);
    int w = u8g2.getStrWidth(name);
    if (w > 124) {
        u8g2.setFont(u8g2_font_helvB14_tr);
        w = u8g2.getStrWidth(name);
    }
    u8g2.drawStr((128 - w) / 2, y, name);
}

static void drawHomePage() {
    if (editMode && editOffDuration) {
        // Stage 2: selecting off duration
        drawHeader("Off Duration");
        char buf[16];
        snprintf(buf, sizeof(buf), "%dh", editOffHours);
        drawLevelCentered(buf, 36);
        u8g2.setFont(u8g2_font_helvR08_tr);
        u8g2.drawStr(12, 52, "rotate: hours  press: ok");
    } else if (editMode) {
        // Stage 1: selecting level
        drawHeader("Set Level");
        drawLevelCentered(getVentilationLevelName(editVentLevel), 38);
        u8g2.setFont(u8g2_font_helvR08_tr);
        u8g2.drawStr(8, 52, "rotate: adjust  press: ok");
    } else if (timedOffActive) {
        // Timed off countdown
        drawHeader("Ventilation");
        drawLevelCentered("Off", 34);
        u8g2.setFont(u8g2_font_helvR08_tr);
        unsigned long rem = getTimedOffRemainingMinutes();
        char buf[32];
        snprintf(buf, sizeof(buf), "Resumes in %luh %lum", rem / 60, rem % 60);
        int w = u8g2.getStrWidth(buf);
        u8g2.drawStr((128 - w) / 2, 52, buf);
    } else {
        drawHeader("Ventilation");
        drawLevelCentered(getVentilationLevelName(cwlData.ventilationLevel), 36);

        u8g2.setFont(u8g2_font_helvR08_tr);
        char buf[40];
        const char* indicator = scheduleOverride ? " M" : (scheduleActive ? " S" : "");
        snprintf(buf, sizeof(buf), "%d%%%s  %s",
                 cwlData.relativeVentilation, indicator,
                 requestedBypassOpen ? "Summer" : "Winter");
        u8g2.drawStr(0, 52, buf);
    }
}

static void drawBypassPage() {
    if (editMode) {
        drawHeader("Set Mode");
        drawLevelCentered(editVentLevel ? "Summer" : "Winter", 34);

        u8g2.setFont(u8g2_font_helvR08_tr);
        u8g2.drawStr(8, 50, "rotate: toggle  press: ok");
    } else {
        drawHeader("Summer Mode");
        drawLevelCentered(requestedBypassOpen ? "Active" : "Inactive", 34);

        u8g2.setFont(u8g2_font_helvR08_tr);
        const char* desc = requestedBypassOpen
            ? "Bypass open - free cooling"
            : "Heat recovery active";
        int w = u8g2.getStrWidth(desc);
        u8g2.drawStr((128 - w) / 2, 52, desc);
    }
}

static void drawTempValue(const char* label, float temp, int y) {
    u8g2.setFont(u8g2_font_helvR08_tr);
    u8g2.drawStr(0, y, label);

    char buf[16];
    snprintf(buf, sizeof(buf), "%.1f C", temp);
    u8g2.setFont(u8g2_font_helvB18_tr);
    u8g2.drawStr(0, y + 20, buf);
}

static void drawTempInPage() {
    drawHeader("Intake");
    drawTempValue("Supply", cwlData.supplyInletTemp, 20);
    drawTempValue("Exhaust", cwlData.exhaustInletTemp, 42);
}

static void drawTempOutPage() {
    drawHeader("Outlet");
    if (cwlData.supportsId81) {
        drawTempValue("Supply", cwlData.supplyOutletTemp, 20);
    }
    if (cwlData.supportsId83) {
        drawTempValue("Exhaust", cwlData.exhaustOutletTemp, cwlData.supportsId81 ? 42 : 20);
    }
    if (cwlData.tspValid[55]) {
        int y = 20;
        if (cwlData.supportsId81) y += 22;
        if (cwlData.supportsId83) y += 22;
        if (y <= 42) {
            drawTempValue("Outdoor", (float)cwlData.tempAtmospheric, y);
        }
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

    #if defined(USE_ETHERNET)
    if (networkConnected) {
        u8g2.drawStr(0, y, "ETH: connected"); y += 11;
        String ip = ETH.localIP().toString();
        u8g2.drawStr(0, y, ip.c_str()); y += 11;
    } else {
        u8g2.drawStr(0, y, "ETH: disconnected"); y += 11;
        y += 11;
    }
    #else
    if (networkConnected) {
        snprintf(buf, sizeof(buf), "WiFi: %ddBm", WiFi.RSSI());
        u8g2.drawStr(0, y, buf); y += 11;
        String ip = WiFi.localIP().toString();
        u8g2.drawStr(0, y, ip.c_str()); y += 11;
    } else {
        u8g2.drawStr(0, y, "WiFi: disconnected"); y += 11;
        y += 11;
    }
    #endif

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

    // Standby: turn off after 5 min idle (stay on during timed off countdown)
    if (!displayStandby && !editMode &&
        millis() - lastActivityTime > STANDBY_TIMEOUT && !timedOffActive) {
        u8g2.setPowerSave(1);
        displayStandby = true;
        return;
    }
    // Wake from standby if IP overlay was requested (from event task context)
    if (displayStandby && overlayActive) {
        displayStandby = false;
        u8g2.setPowerSave(0);
    }
    if (displayStandby) return;  // Skip rendering while in standby

    // Clear expired IP overlay
    if (overlayActive && millis() - overlayStart > OVERLAY_TIMEOUT) {
        overlayActive = false;
    }

    u8g2.clearBuffer();

    if (overlayActive && !editMode) {
        // Network status overlay screen (timed, 10s)
        u8g2.setFont(u8g2_font_helvR08_tr);
        int hw = u8g2.getStrWidth(overlayHeader);
        u8g2.drawStr((128 - hw) / 2, 18, overlayHeader);

        u8g2.setFont(u8g2_font_helvB14_tr);
        int mw = u8g2.getStrWidth(overlayMessage);
        u8g2.drawStr((128 - mw) / 2, 42, overlayMessage);
    } else if (!cwlData.connected && !editMode) {
        // Permanent CWL disconnected screen
        u8g2.setFont(u8g2_font_helvR08_tr);
        const char* hdr = "CWL";
        int hw = u8g2.getStrWidth(hdr);
        u8g2.drawStr((128 - hw) / 2, 18, hdr);

        u8g2.setFont(u8g2_font_helvB14_tr);
        const char* msg = "Disconnected";
        int mw = u8g2.getStrWidth(msg);
        u8g2.drawStr((128 - mw) / 2, 42, msg);
    } else {
        switch (currentPage) {
            case PAGE_HOME:     drawHomePage(); break;
            case PAGE_BYPASS:   drawBypassPage(); break;
            case PAGE_TEMP_IN:  drawTempInPage(); break;
            case PAGE_TEMP_OUT: drawTempOutPage(); break;
            case PAGE_STATUS:   drawStatusPage(); break;
            case PAGE_SYSTEM:   drawSystemPage(); break;
        }
    }

    // Page indicator dots (centered, y=61 so radius 2 stays within 64px)
    int dotsWidth = (PAGE_COUNT - 1) * 7;
    int dotsStart = (128 - dotsWidth) / 2;
    for (int i = 0; i < PAGE_COUNT; i++) {
        int x = dotsStart + i * 7;
        if (i == currentPage) {
            u8g2.drawDisc(x, 61, 2);
        } else {
            u8g2.drawCircle(x, 61, 2);
        }
    }

    #ifdef SIMULATE_OT
    u8g2.setFont(u8g2_font_helvR08_tr);
    u8g2.drawStr(128 - u8g2.getStrWidth("SIM"), 8, "SIM");
    #endif

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
        editOffDuration = false;
        editVentLevel = timedOffActive ? VENT_LEVEL_OFF : cwlData.ventilationLevel;
        editOffHours = 1;
        editModeStart = millis();
    } else if (currentPage == PAGE_BYPASS) {
        editMode = true;
        editOffDuration = false;
        editVentLevel = requestedBypassOpen ? 1 : 0;
        editModeStart = millis();
    }
}

void exitEditMode(bool apply) {
    if (apply) {
        if (currentPage == PAGE_HOME) {
            if (editOffDuration) {
                // Confirm timed off
                activateTimedOff(editOffHours);
                log("Display: Timed off for " + String(editOffHours) + "h");
            } else if (editVentLevel == VENT_LEVEL_OFF) {
                // Selected Off but didn't enter duration — enter duration substage
                editOffDuration = true;
                editOffHours = 1;
                editModeStart = millis();
                return;  // Don't exit edit mode
            } else {
                // Non-off level: cancel timed off if active, set level
                if (timedOffActive) cancelTimedOff();
                requestedVentLevel = editVentLevel;
                appConfig.ventilationLevel = editVentLevel;
                setVentilationManualOverride();
                log("Display: Ventilation set to " + String(editVentLevel) +
                    " (" + getVentilationLevelName(editVentLevel) + ")");
            }
        } else if (currentPage == PAGE_BYPASS) {
            bool open = editVentLevel != 0;
            requestedBypassOpen = open;
            appConfig.bypassOpen = open;
            saveBypassState();
            setBypassManualOverride();
            log("Display: Bypass set to " + String(open ? "summer (open)" : "winter (closed)"));
        }
    }
    editMode = false;
    editOffDuration = false;
}

void adjustEditValue(int delta) {
    if (!editMode) return;
    editModeStart = millis();

    if (currentPage == PAGE_HOME) {
        if (editOffDuration) {
            // Adjusting off duration hours
            int newHours = (int)editOffHours + delta;
            if (newHours < 1) {
                // Rotate back past 1h — exit duration substage, go to Party
                editOffDuration = false;
                editVentLevel = VENT_LEVEL_PARTY;
            } else if (newHours > 99) {
                editOffHours = 99;
            } else {
                editOffHours = newHours;
            }
        } else {
            // Cycling through levels 0-3
            int newLevel = (int)editVentLevel + delta;
            if (newLevel < 0) newLevel = 3;
            if (newLevel > 3) newLevel = 0;
            editVentLevel = newLevel;
        }
    } else if (currentPage == PAGE_BYPASS) {
        editVentLevel = editVentLevel ? 0 : 1;
    }
}

bool displayWake() {
    lastActivityTime = millis();
    overlayActive = false;  // User interaction dismisses IP overlay
    if (displayStandby) {
        displayStandby = false;
        u8g2.setPowerSave(0);
        return true;  // Input consumed — caller should discard
    }
    return false;  // Was already awake — pass input through
}

// Safe to call from any context (event task, ISR) — only sets flags,
// no I2C. The actual wake happens in updateDisplay() on the main loop.
static void showOverlay(const char* header, const char* message) {
    strncpy(overlayHeader, header, sizeof(overlayHeader) - 1);
    overlayHeader[sizeof(overlayHeader) - 1] = '\0';
    strncpy(overlayMessage, message, sizeof(overlayMessage) - 1);
    overlayMessage[sizeof(overlayMessage) - 1] = '\0';
    overlayStart = millis();
    overlayActive = true;
    lastActivityTime = millis();
}

void displayShowIP(const char* ip) {
    showOverlay("Connected", ip);
}

void displayShowDisconnected() {
    showOverlay("Disconnected", "No network");
}
