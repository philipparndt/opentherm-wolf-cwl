#pragma once

#include <Arduino.h>

#define MAX_SCHEDULES 16

// Schedule entry
struct ScheduleEntry {
    bool enabled;
    uint8_t startHour;    // 0-23
    uint8_t startMinute;  // 0-59
    uint8_t endHour;      // 0-23
    uint8_t endMinute;    // 0-59
    uint8_t ventLevel;    // 0-3
    uint8_t activeDays;   // Bitmask: bit0=Mon, bit1=Tue, ..., bit6=Sun
};

// Bypass date-range schedule
struct BypassSchedule {
    bool enabled;
    uint8_t startDay;     // 1-31
    uint8_t startMonth;   // 1-12
    uint8_t endDay;       // 1-31
    uint8_t endMonth;     // 1-12
};

// Global schedule state
extern ScheduleEntry schedules[MAX_SCHEDULES];
extern int scheduleCount;
extern BypassSchedule bypassSchedule;
extern bool scheduleActive;       // A ventilation schedule is controlling the level
extern bool scheduleOverride;     // Manual override is active
extern bool bypassScheduleActive; // Bypass schedule is controlling bypass
extern bool bypassOverride;       // Bypass manual override

// Scheduler functions
void setupScheduler();
void schedulerLoop();

// Timed off mode
extern bool timedOffActive;
extern time_t timedOffEndEpoch;   // UTC epoch when timed off expires (0 = inactive)
extern uint8_t timedOffHours;

void activateTimedOff(uint8_t hours);
void cancelTimedOff();
unsigned long getTimedOffRemainingMinutes();

// Ventilation schedule manual override
void setVentilationManualOverride();

// Bypass manual override
void setBypassManualOverride();

// NVS persistence
void loadSchedules();
void saveSchedules();
void loadBypassSchedule();
void saveBypassSchedule();

// JSON serialization for web API
String getSchedulesJson();
bool updateSchedulesFromJson(const String& json);
String getBypassScheduleJson();
bool updateBypassScheduleFromJson(const String& json);

// Get description of active schedule
String getActiveScheduleDescription();
