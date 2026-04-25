#include "wolf_cwl.h"

CwlData cwlData;

void initCwlData() {
    memset(&cwlData, 0, sizeof(CwlData));
    cwlData.ventilationLevel = 2;  // Normal
    cwlData.connected = false;
    cwlData.lastResponse = 0;
}

void updateDerivedValues() {
    // Current volume (TSP 52=LO, 53=HI)
    if (cwlData.tspValid[52] && cwlData.tspValid[53]) {
        cwlData.currentVolume = cwlData.tspValues[52] | (cwlData.tspValues[53] << 8);
    }

    // Bypass status (TSP 54)
    if (cwlData.tspValid[54]) {
        cwlData.bypassStatus = cwlData.tspValues[54];
    }

    // Atmospheric temperature (TSP 55, value - 100 = °C)
    if (cwlData.tspValid[55]) {
        cwlData.tempAtmospheric = (int8_t)(cwlData.tspValues[55] - 100);
    }

    // Indoor temperature (TSP 56, value - 100 = °C)
    if (cwlData.tspValid[56]) {
        cwlData.tempIndoor = (int8_t)(cwlData.tspValues[56] - 100);
    }

    // Input duct pressure (TSP 64=LO, 65=HI)
    if (cwlData.tspValid[64] && cwlData.tspValid[65]) {
        cwlData.inputDuctPressure = cwlData.tspValues[64] | (cwlData.tspValues[65] << 8);
    }

    // Output duct pressure (TSP 66=LO, 67=HI)
    if (cwlData.tspValid[66] && cwlData.tspValid[67]) {
        cwlData.outputDuctPressure = cwlData.tspValues[66] | (cwlData.tspValues[67] << 8);
    }

    // Frost status (TSP 68)
    if (cwlData.tspValid[68]) {
        cwlData.frostStatus = cwlData.tspValues[68];
    }
}

const char* getVentilationLevelName(uint8_t level) {
    switch (level) {
        case VENT_LEVEL_OFF:     return "Off";
        case VENT_LEVEL_REDUCED: return "Reduced";
        case VENT_LEVEL_NORMAL:  return "Normal";
        case VENT_LEVEL_PARTY:   return "Party";
        default:                 return "Unknown";
    }
}

float decodeF88(uint16_t value) {
    int8_t intPart = (int8_t)(value >> 8);
    uint8_t fracPart = value & 0xFF;
    return intPart + fracPart / 256.0f;
}
