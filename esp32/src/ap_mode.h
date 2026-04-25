#pragma once

#include <Arduino.h>

// AP mode state
extern bool apModeActive;

// AP mode functions
bool shouldStartApMode();
void setupApMode();
void stopApMode();
void apModeLoop();
