#include "scheduler.h"
#include "ot_master.h"
#include "config_manager.h"
#include "wolf_cwl.h"
#include "logging.h"
#include "display.h"
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

// Timed off state
bool timedOffActive = false;
time_t timedOffEndEpoch = 0;
uint8_t timedOffHours = 0;
static bool timedOffRestoredFromNvs = false;  // Deferred NTP check pending

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

    // Restore persisted override state (written only on manual user actions)
    Preferences prefs;
    prefs.begin(PREFS_NAMESPACE, true);

    // Timed off
    uint32_t storedEnd = prefs.getUInt("toff_end", 0);
    if (storedEnd > 0) {
        timedOffEndEpoch = (time_t)storedEnd;
        timedOffActive = true;
        timedOffRestoredFromNvs = true;  // Will verify against NTP in schedulerLoop
        requestedVentLevel = VENT_LEVEL_OFF;
        scheduleOverride = true;
        log("Scheduler: Restored timed off from NVS (end=" + String(storedEnd) + ")");
    }

    // Manual overrides (only if not already in timed off)
    if (!timedOffActive) {
        scheduleOverride = prefs.getBool("sched_ovr", false);
        if (scheduleOverride) {
            log("Scheduler: Restored ventilation manual override from NVS");
        }
    }
    bypassOverride = prefs.getBool("bp_ovr", false);
    if (bypassOverride) {
        log("Scheduler: Restored bypass manual override from NVS");
    }

    prefs.end();
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
static void persistOverrideState() {
    Preferences prefs;
    prefs.begin(PREFS_NAMESPACE, false);
    prefs.putUInt("toff_end", timedOffActive ? (uint32_t)timedOffEndEpoch : 0);
    prefs.putBool("sched_ovr", scheduleOverride && !timedOffActive);
    prefs.putBool("bp_ovr", bypassOverride);
    prefs.end();
}

void schedulerLoop() {
    unsigned long now = millis();

    // --- Timed off: NTP-deferred validation after reboot ---
    time_t now_time = time(nullptr);
    bool ntpSynced = now_time > 1700000000;

    if (timedOffActive && timedOffRestoredFromNvs && ntpSynced) {
        timedOffRestoredFromNvs = false;
        if (timedOffEndEpoch <= now_time) {
            // Timer already expired during downtime
            log("Scheduler: Timed off expired during downtime, resuming Normal");
            timedOffActive = false;
            timedOffHours = 0;
            timedOffEndEpoch = 0;
            requestedVentLevel = VENT_LEVEL_NORMAL;
            scheduleOverride = false;
            persistOverrideState();
            displayWake();
        } else {
            timedOffHours = (uint8_t)((timedOffEndEpoch - now_time) / 3600 + 1);
            log("Scheduler: Timed off restored, " + String(timedOffHours) + "h remaining");
        }
    }

    // --- Timed off expiry check (epoch-based) ---
    if (timedOffActive && !timedOffRestoredFromNvs && ntpSynced && now_time >= timedOffEndEpoch) {
        log("Scheduler: Timed off expired, resuming Normal");
        timedOffActive = false;
        timedOffHours = 0;
        timedOffEndEpoch = 0;
        requestedVentLevel = VENT_LEVEL_NORMAL;
        scheduleOverride = false;
        persistOverrideState();
        displayWake();
    }

    if (now - lastEvalTime < EVAL_INTERVAL) return;
    lastEvalTime = now;

    if (!ntpSynced) return;

    struct tm timeinfo;
    localtime_r(&now_time, &timeinfo);

    // --- Ventilation schedule (skip if timed off is active) ---
    if (!timedOffActive) {
        int matchIndex = evaluateVentilationSchedule(&timeinfo);

        if (matchIndex != lastActiveScheduleIndex) {
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
                    displayWake();
                    log("Scheduler: Level set to " + String(level) +
                        " (" + String(getVentilationLevelName(level)) + ")");
                }
            } else {
                if (scheduleActive) {
                    scheduleActive = false;
                    requestedVentLevel = appConfig.ventilationLevel;
                    displayWake();
                    log("Scheduler: No schedule active, using default level " +
                        String(appConfig.ventilationLevel));
                }
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
            displayWake();
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
        persistOverrideState();
        log("Scheduler: Ventilation manual override set (persisted)");
    }
}

void setBypassManualOverride() {
    if (bypassScheduleActive) {
        bypassOverride = true;
        persistOverrideState();
        log("Scheduler: Bypass manual override set (persisted)");
    }
}

// =============================================================================
// Timed off mode
// =============================================================================
void activateTimedOff(uint8_t hours) {
    if (hours == 0 || hours > 99) return;
    time_t now_time = time(nullptr);
    if (now_time < 1700000000) return;  // Need NTP for epoch

    timedOffActive = true;
    timedOffHours = hours;
    timedOffEndEpoch = now_time + (time_t)hours * 3600;
    timedOffRestoredFromNvs = false;
    requestedVentLevel = VENT_LEVEL_OFF;
    scheduleOverride = true;
    persistOverrideState();
    displayWake();
    log("Scheduler: Timed off activated for " + String(hours) + "h (persisted)");
}

void cancelTimedOff() {
    if (!timedOffActive) return;
    timedOffActive = false;
    timedOffHours = 0;
    timedOffEndEpoch = 0;
    timedOffRestoredFromNvs = false;
    requestedVentLevel = VENT_LEVEL_NORMAL;
    scheduleOverride = false;
    persistOverrideState();
    displayWake();
    log("Scheduler: Timed off cancelled, resuming Normal (persisted)");
}

unsigned long getTimedOffRemainingMinutes() {
    if (!timedOffActive) return 0;
    time_t now_time = time(nullptr);
    if (now_time < 1700000000) return 0;  // NTP not synced
    if (now_time >= timedOffEndEpoch) return 0;
    return (unsigned long)(timedOffEndEpoch - now_time) / 60;
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
static const char* daysLabel(uint8_t mask) {
    if (mask == 0x7F) return "Every day";
    if (mask == 0x1F) return "Weekdays";
    if (mask == 0x60) return "Weekend";
    return nullptr;  // Use individual days
}

String getActiveScheduleDescription() {
    if (!scheduleActive || lastActiveScheduleIndex < 0) return "";

    ScheduleEntry& s = schedules[lastActiveScheduleIndex];
    const char* days = daysLabel(s.activeDays);
    char buf[64];
    if (days) {
        snprintf(buf, sizeof(buf), "%02d:%02d-%02d:%02d %s %s",
                 s.startHour, s.startMinute, s.endHour, s.endMinute,
                 getVentilationLevelName(s.ventLevel), days);
    } else {
        snprintf(buf, sizeof(buf), "%02d:%02d-%02d:%02d %s",
                 s.startHour, s.startMinute, s.endHour, s.endMinute,
                 getVentilationLevelName(s.ventLevel));
    }
    return String(buf);
}
