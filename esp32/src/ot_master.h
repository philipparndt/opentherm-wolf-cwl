#pragma once

#include <Arduino.h>

// OpenTherm state
extern bool otConnected;

// Control requests (set by MQTT/encoder, applied in next polling cycle)
extern uint8_t requestedVentLevel;
extern bool requestedBypassOpen;
extern bool requestedFilterReset;

// OpenTherm functions
void setupOpenTherm();
void openThermLoop();

// Probe additional data IDs at startup
void probeAdditionalIds();
