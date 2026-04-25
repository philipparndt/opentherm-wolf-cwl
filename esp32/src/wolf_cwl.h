#pragma once

#include <Arduino.h>

// Ventilation level names
#define VENT_LEVEL_OFF      0
#define VENT_LEVEL_REDUCED  1
#define VENT_LEVEL_NORMAL   2
#define VENT_LEVEL_PARTY    3

// TSP register count (indices 0-68)
#define TSP_MAX_INDEX 69

// Wolf CWL data model
struct CwlData {
    // Temperatures (f8.8 format, °C)
    float supplyInletTemp;     // ID 80
    float exhaustInletTemp;    // ID 82
    float supplyOutletTemp;    // ID 81 (if supported)
    float exhaustOutletTemp;   // ID 83 (if supported)

    // Ventilation
    uint8_t ventilationLevel;  // ID 71: 0-3 (Off/Reduced/Normal/Party)
    uint8_t relativeVentilation; // ID 77: 0-100%

    // Status flags from ID 70 response (slave → master)
    bool fault;
    bool ventilationActive;    // bypass active
    bool coolingActive;
    bool dhwActive;
    bool filterDirty;          // filter needs replacement
    bool diagEvent;

    // Fault info from ID 72
    uint8_t faultFlags;
    uint8_t oemFaultCode;

    // Config from ID 74
    uint8_t slaveFlags;
    uint8_t slaveMemberId;

    // Fan speeds (if supported)
    uint16_t exhaustFanSpeed;  // ID 84
    uint16_t supplyFanSpeed;   // ID 85

    // TSP register cache
    uint8_t tspValues[TSP_MAX_INDEX];
    bool tspValid[TSP_MAX_INDEX];

    // Product versions
    uint8_t masterType;
    uint8_t masterVersion;
    uint8_t slaveType;
    uint8_t slaveVersion;

    // Probed data IDs (which additional IDs are supported)
    bool supportsId78;   // Relative humidity exhaust
    bool supportsId79;   // CO2 level
    bool supportsId81;   // Supply outlet temp
    bool supportsId83;   // Exhaust outlet temp
    bool supportsId84;   // Exhaust fan speed
    bool supportsId85;   // Supply fan speed
    bool supportsId87;   // Nominal ventilation
    bool supportsId88;   // TSP count

    // Derived values from TSP registers
    uint16_t currentVolume;    // TSP 52,53: m³/h
    uint8_t bypassStatus;     // TSP 54: 0=shut, 1=auto, 2=open
    int8_t tempAtmospheric;   // TSP 55: °C (value - 100)
    int8_t tempIndoor;        // TSP 56: °C (value - 100)
    uint16_t inputDuctPressure;  // TSP 64,65: Pa
    uint16_t outputDuctPressure; // TSP 66,67: Pa
    uint8_t frostStatus;      // TSP 68

    // Connection state
    bool connected;            // true if CWL is responding
    unsigned long lastResponse; // millis() of last valid response
};

// Global data model
extern CwlData cwlData;

// Initialize data model
void initCwlData();

// Update derived values from TSP cache
void updateDerivedValues();

// Get ventilation level name
const char* getVentilationLevelName(uint8_t level);

// Decode f8.8 temperature from OpenTherm value
float decodeF88(uint16_t value);
