#pragma once

#include <Arduino.h>

// Watchdog functions
void setupWatchdog();
void feedWatchdog();

// Diagnostics
const char* getRebootReasonStr();
uint32_t getCrashCount();
unsigned long getUptimeSeconds();
unsigned long getOtResponseAge();
