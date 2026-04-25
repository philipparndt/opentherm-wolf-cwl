#include "scheduler.h"
#include "ot_master.h"
#include "config_manager.h"
#include "wolf_cwl.h"
#include "logging.h"
#include <Preferences.h>
#include <ArduinoJson.h>
#include <time.h>

// =============================================================================
// Global state
// =============================================================================
ScheduleEntry schedules[MAX_SCHEDULES];
int scheduleCount = 0;
BypassSchedule bypassSchedule;

bool scheduleActive = false;
bool scheduleOverride = false;
bool bypassScheduleActive = false;
bool bypassOverride = false;

static int lastActiveScheduleIndex = -1;
static bool lastBypassSeason = false;
static unsigned long lastEvalTime = 0;
static bool bypassSeasonInitialized = false;

#define EVAL_INTERVAL 60000  // Evaluate once per minute

static const char* PREFS_NAMESPACE = "wolfcwl";

// =============================================================================
// NVS persistence — Ventilation schedules
// =============================================================================
void loadSchedules() {
    Preferences prefs;
    prefs.begin(PREFS_NAMESPACE, true);
    scheduleCount = prefs.getInt("sched_count", 0);
    if (scheduleCount > MAX_SCHEDULES) scheduleCount = MAX_SCHEDULES;

    for (int i = 0; i < scheduleCount; i++) {
        String p = "sched_" + String(i) + "_";
        schedules[i].enabled = prefs.getBool((p + "en").c_str(), false);
        schedules[i].startHour = prefs.getInt((p + "sh").c_str(), 0);
        schedules[i].startMinute = prefs.getInt((p + "sm").c_str(), 0);
        schedules[i].endHour = prefs.getInt((p + "eh").c_str(), 0);
        schedules[i].endMinute = prefs.getInt((p + "em").c_str(), 0);
        schedules[i].ventLevel = prefs.getInt((p + "lv").c_str(), 2);
        schedules[i].activeDays = prefs.getInt((p + "dy").c_str(), 0x7F);  // All days
    }
    prefs.end();

    if (scheduleCount > 0) {
        log("Scheduler: Loaded " + String(scheduleCount) + " ventilation schedules");
    }
}

void saveSchedules() {
    Preferences prefs;
    prefs.begin(PREFS_NAMESPACE, false);
    prefs.putInt("sched_count", scheduleCount);

    for (int i = 0; i < MAX_SCHEDULES; i++) {
        String p = "sched_" + String(i) + "_";
        if (i < scheduleCount) {
            prefs.putBool((p + "en").c_str(), schedules[i].enabled);
            prefs.putInt((p + "sh").c_str(), schedules[i].startHour);
            prefs.putInt((p + "sm").c_str(), schedules[i].startMinute);
            prefs.putInt((p + "eh").c_str(), schedules[i].endHour);
            prefs.putInt((p + "em").c_str(), schedules[i].endMinute);
            prefs.putInt((p + "lv").c_str(), schedules[i].ventLevel);
            prefs.putInt((p + "dy").c_str(), schedules[i].activeDays);
        } else if (prefs.isKey((p + "en").c_str())) {
            prefs.remove((p + "en").c_str());
            prefs.remove((p + "sh").c_str());
            prefs.remove((p + "sm").c_str());
            prefs.remove((p + "eh").c_str());
            prefs.remove((p + "em").c_str());
            prefs.remove((p + "lv").c_str());
            prefs.remove((p + "dy").c_str());
        }
    }
    prefs.end();
    log("Scheduler: Saved " + String(scheduleCount) + " ventilation schedules");
}

// =============================================================================
// NVS persistence — Bypass schedule
// =============================================================================
void loadBypassSchedule() {
    Preferences prefs;
    prefs.begin(PREFS_NAMESPACE, true);
    bypassSchedule.enabled = prefs.getBool("bp_sched_en", false);
    bypassSchedule.startDay = prefs.getInt("bp_sched_sd", 15);
    bypassSchedule.startMonth = prefs.getInt("bp_sched_sm", 4);  // April
    bypassSchedule.endDay = prefs.getInt("bp_sched_ed", 30);
    bypassSchedule.endMonth = prefs.getInt("bp_sched_em", 9);    // September
    prefs.end();

    if (bypassSchedule.enabled) {
        log("Scheduler: Bypass schedule loaded (" +
            String(bypassSchedule.startDay) + "." + String(bypassSchedule.startMonth) +
            " - " + String(bypassSchedule.endDay) + "." + String(bypassSchedule.endMonth) + ")");
    }
}

void saveBypassSchedule() {
    Preferences prefs;
    prefs.begin(PREFS_NAMESPACE, false);
    prefs.putBool("bp_sched_en", bypassSchedule.enabled);
    prefs.putInt("bp_sched_sd", bypassSchedule.startDay);
    prefs.putInt("bp_sched_sm", bypassSchedule.startMonth);
    prefs.putInt("bp_sched_ed", bypassSchedule.endDay);
    prefs.putInt("bp_sched_em", bypassSchedule.endMonth);
    prefs.end();
    log("Scheduler: Bypass schedule saved");
}

// =============================================================================
// Setup
// =============================================================================
void setupScheduler() {
    memset(schedules, 0, sizeof(schedules));
    memset(&bypassSchedule, 0, sizeof(bypassSchedule));
    loadSchedules();
    loadBypassSchedule();
}

// =============================================================================
// Time matching helpers
// =============================================================================
static int timeToMinutes(uint8_t h, uint8_t m) {
    return h * 60 + m;
}

// Convert tm_wday (0=Sun) to our bitmask (bit0=Mon)
static uint8_t tmWdayToBitmask(int wday) {
    // wday: 0=Sun, 1=Mon, ..., 6=Sat
    // our bitmask: bit0=Mon, bit1=Tue, ..., bit5=Sat, bit6=Sun
    if (wday == 0) return (1 << 6);  // Sunday = bit 6
    return (1 << (wday - 1));
}

static bool isTimeInRange(int currentMinutes, int startMinutes, int endMinutes) {
    if (startMinutes <= endMinutes) {
        // Normal range (e.g., 08:00 - 22:00)
        return currentMinutes >= startMinutes && currentMinutes < endMinutes;
    } else {
        // Spans midnight (e.g., 22:00 - 06:00)
        return currentMinutes >= startMinutes || currentMinutes < endMinutes;
    }
}

// Check if a date (day, month) falls within [startDay.startMonth, endDay.endMonth]
static bool isDateInRange(int day, int month, int sd, int sm, int ed, int em) {
    int current = month * 100 + day;
    int start = sm * 100 + sd;
    int end = em * 100 + ed;

    if (start <= end) {
        // Normal range (e.g., Apr 15 - Sep 30)
        return current >= start && current <= end;
    } else {
        // Spans year boundary (e.g., Oct 1 - Mar 31)
        return current >= start || current <= end;
    }
}

// =============================================================================
// Ventilation schedule evaluation
// =============================================================================
static int evaluateVentilationSchedule(struct tm* timeinfo) {
    int currentMinutes = timeToMinutes(timeinfo->tm_hour, timeinfo->tm_min);
    uint8_t dayBit = tmWdayToBitmask(timeinfo->tm_wday);

    for (int i = 0; i < scheduleCount; i++) {
        if (!schedules[i].enabled) continue;
        if (!(schedules[i].activeDays & dayBit)) continue;

        int startMin = timeToMinutes(schedules[i].startHour, schedules[i].startMinute);
        int endMin = timeToMinutes(schedules[i].endHour, schedules[i].endMinute);

        if (isTimeInRange(currentMinutes, startMin, endMin)) {
            return i;
        }
    }
    return -1;  // No match
}

// =============================================================================
// Bypass schedule evaluation
// =============================================================================
static bool evaluateBypassSchedule(struct tm* timeinfo) {
    if (!bypassSchedule.enabled) return false;

    int day = timeinfo->tm_mday;
    int month = timeinfo->tm_mon + 1;  // tm_mon is 0-based

    return isDateInRange(day, month,
                         bypassSchedule.startDay, bypassSchedule.startMonth,
                         bypassSchedule.endDay, bypassSchedule.endMonth);
}

// =============================================================================
// Main scheduler loop
// =============================================================================
void schedulerLoop() {
    unsigned long now = millis();
    if (now - lastEvalTime < EVAL_INTERVAL) return;
    lastEvalTime = now;

    // Check NTP sync
    time_t now_time = time(nullptr);
    if (now_time < 1700000000) return;  // NTP not synced

    struct tm timeinfo;
    localtime_r(&now_time, &timeinfo);

    // --- Ventilation schedule ---
    int matchIndex = evaluateVentilationSchedule(&timeinfo);

    if (matchIndex != lastActiveScheduleIndex) {
        // Transition detected — clear override
        if (scheduleOverride) {
            scheduleOverride = false;
            log("Scheduler: Override cleared (schedule transition)");
        }
        lastActiveScheduleIndex = matchIndex;
    }

    if (!scheduleOverride) {
        if (matchIndex >= 0) {
            scheduleActive = true;
            uint8_t level = schedules[matchIndex].ventLevel;
            if (level != requestedVentLevel) {
                requestedVentLevel = level;
                log("Scheduler: Level set to " + String(level) +
                    " (" + String(getVentilationLevelName(level)) + ")");
            }
        } else {
            if (scheduleActive) {
                // No schedule matches — revert to default
                scheduleActive = false;
                requestedVentLevel = appConfig.ventilationLevel;
                log("Scheduler: No schedule active, using default level " +
                    String(appConfig.ventilationLevel));
            }
        }
    }

    // --- Bypass schedule ---
    bool isSummer = evaluateBypassSchedule(&timeinfo);

    if (bypassSeasonInitialized && isSummer != lastBypassSeason) {
        // Season transition — clear override
        if (bypassOverride) {
            bypassOverride = false;
            log("Scheduler: Bypass override cleared (season transition)");
        }
    }
    lastBypassSeason = isSummer;
    bypassSeasonInitialized = true;

    if (bypassSchedule.enabled && !bypassOverride) {
        bypassScheduleActive = true;
        if (isSummer != requestedBypassOpen) {
            requestedBypassOpen = isSummer;
            log("Scheduler: Bypass " + String(isSummer ? "open (summer)" : "closed (winter)"));
        }
    } else if (!bypassSchedule.enabled) {
        bypassScheduleActive = false;
    }
}

// =============================================================================
// Manual override
// =============================================================================
void setVentilationManualOverride() {
    if (scheduleActive) {
        scheduleOverride = true;
        log("Scheduler: Ventilation manual override set");
    }
}

void setBypassManualOverride() {
    if (bypassScheduleActive) {
        bypassOverride = true;
        log("Scheduler: Bypass manual override set");
    }
}

// =============================================================================
// JSON serialization — Ventilation schedules
// =============================================================================
String getSchedulesJson() {
    JsonDocument doc;
    JsonArray arr = doc.to<JsonArray>();

    for (int i = 0; i < scheduleCount; i++) {
        JsonObject entry = arr.add<JsonObject>();
        entry["enabled"] = schedules[i].enabled;
        entry["startHour"] = schedules[i].startHour;
        entry["startMinute"] = schedules[i].startMinute;
        entry["endHour"] = schedules[i].endHour;
        entry["endMinute"] = schedules[i].endMinute;
        entry["ventLevel"] = schedules[i].ventLevel;
        entry["activeDays"] = schedules[i].activeDays;
    }

    String output;
    serializeJson(doc, output);
    return output;
}

bool updateSchedulesFromJson(const String& json) {
    JsonDocument doc;
    DeserializationError error = deserializeJson(doc, json);
    if (error) {
        log("Scheduler: JSON parse error: " + String(error.c_str()));
        return false;
    }

    if (!doc.is<JsonArray>()) return false;
    JsonArray arr = doc.as<JsonArray>();

    scheduleCount = 0;
    for (JsonObject entry : arr) {
        if (scheduleCount >= MAX_SCHEDULES) break;
        int i = scheduleCount;
        schedules[i].enabled = entry["enabled"] | false;
        schedules[i].startHour = entry["startHour"] | 0;
        schedules[i].startMinute = entry["startMinute"] | 0;
        schedules[i].endHour = entry["endHour"] | 0;
        schedules[i].endMinute = entry["endMinute"] | 0;
        schedules[i].ventLevel = entry["ventLevel"] | 2;
        schedules[i].activeDays = entry["activeDays"] | 0x7F;
        scheduleCount++;
    }

    saveSchedules();
    return true;
}

// =============================================================================
// JSON serialization — Bypass schedule
// =============================================================================
String getBypassScheduleJson() {
    JsonDocument doc;
    doc["enabled"] = bypassSchedule.enabled;
    doc["startDay"] = bypassSchedule.startDay;
    doc["startMonth"] = bypassSchedule.startMonth;
    doc["endDay"] = bypassSchedule.endDay;
    doc["endMonth"] = bypassSchedule.endMonth;

    String output;
    serializeJson(doc, output);
    return output;
}

bool updateBypassScheduleFromJson(const String& json) {
    JsonDocument doc;
    DeserializationError error = deserializeJson(doc, json);
    if (error) return false;

    bypassSchedule.enabled = doc["enabled"] | false;
    bypassSchedule.startDay = doc["startDay"] | 15;
    bypassSchedule.startMonth = doc["startMonth"] | 4;
    bypassSchedule.endDay = doc["endDay"] | 30;
    bypassSchedule.endMonth = doc["endMonth"] | 9;

    saveBypassSchedule();
    return true;
}

// =============================================================================
// Active schedule description
// =============================================================================
String getActiveScheduleDescription() {
    if (!scheduleActive || lastActiveScheduleIndex < 0) return "";

    ScheduleEntry& s = schedules[lastActiveScheduleIndex];
    char buf[48];
    snprintf(buf, sizeof(buf), "%02d:%02d-%02d:%02d %s",
             s.startHour, s.startMinute, s.endHour, s.endMinute,
             getVentilationLevelName(s.ventLevel));
    return String(buf);
}
