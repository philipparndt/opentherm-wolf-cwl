#include "ot_master.h"
#include "config_manager.h"
#include "wolf_cwl.h"
#include "logging.h"

bool otConnected = false;

// Control state (written by MQTT/encoder, read by polling cycle)
uint8_t requestedVentLevel = 2;   // Normal
bool requestedBypassOpen = false;
bool requestedFilterReset = false;

// =============================================================================
// SIMULATION MODE — fake OpenTherm data for testing without hardware
// =============================================================================
#ifdef SIMULATE_OT

static unsigned long lastSimTime = 0;
static bool simInitialized = false;

void setupOpenTherm() {
    log("OpenTherm: *** SIMULATION MODE ***");
    requestedVentLevel = appConfig.ventilationLevel;
    requestedBypassOpen = appConfig.bypassOpen;

    // Mark all additional IDs as supported
    cwlData.supportsId81 = true;
    cwlData.supportsId83 = true;
    cwlData.supportsId84 = true;
    cwlData.supportsId85 = true;

    // Populate static TSP values (realistic Wolf CWL defaults)
    uint8_t tspDefaults[][2] = {
        {0, 100}, {1, 0},    // U1: Reduced airflow 100 m³/h
        {2, 130}, {3, 0},    // U2: Normal airflow 130 m³/h
        {4, 195}, {5, 0},    // U3: Party airflow 195 m³/h
        {6, 20},             // U4: Min outdoor temp for bypass (10°C)
        {7, 44},             // U5: Min indoor temp for bypass (22°C)
        {11, 100},           // I1: Imbalance 0%
        {17, 1}, {18, 1}, {19, 2},
        {23, 1}, {24, 0},
        {54, 1},             // Bypass status: auto
        {68, 0},             // Frost: none
    };
    for (auto& tsp : tspDefaults) {
        cwlData.tspValues[tsp[0]] = tsp[1];
        cwlData.tspValid[tsp[0]] = true;
    }

    cwlData.slaveFlags = 0x07;
    cwlData.slaveMemberId = 0;
    cwlData.slaveType = 0;
    cwlData.slaveVersion = 0;
    cwlData.connected = true;
    cwlData.lastResponse = millis();

    simInitialized = true;
    log("OpenTherm: Simulation initialized with fake CWL data");
}

void probeAdditionalIds() {
    // Already done in setup
}

void openThermLoop() {
    unsigned long now = millis();
    if (now - lastSimTime < 1000) return;
    lastSimTime = now;

    if (!simInitialized) return;

    cwlData.lastResponse = now;

    // Oscillating temperatures
    float t = now / 1000.0f;
    cwlData.supplyInletTemp = 18.0f + 5.0f * sin(t / 60.0f);
    cwlData.exhaustInletTemp = 21.0f + 2.0f * sin(t / 45.0f);
    cwlData.supplyOutletTemp = 20.0f + 3.0f * sin(t / 50.0f);
    cwlData.exhaustOutletTemp = 16.0f + 4.0f * sin(t / 55.0f);

    // Ventilation responds to level
    cwlData.ventilationLevel = requestedVentLevel;
    static const uint8_t ventMap[] = {0, 51, 67, 100};
    cwlData.relativeVentilation = ventMap[requestedVentLevel > 3 ? 2 : requestedVentLevel];

    // Status flags
    cwlData.fault = false;
    cwlData.ventilationActive = requestedBypassOpen;
    cwlData.filterDirty = false;
    cwlData.coolingActive = false;
    cwlData.dhwActive = false;
    cwlData.diagEvent = false;
    cwlData.faultFlags = 0;
    cwlData.oemFaultCode = 0;

    // Fan speeds
    uint16_t baseSpeed = cwlData.relativeVentilation * 25;
    cwlData.exhaustFanSpeed = baseSpeed + (uint16_t)(50 * sin(t / 30.0f));
    cwlData.supplyFanSpeed = baseSpeed + (uint16_t)(40 * sin(t / 35.0f));

    // Dynamic TSP values
    uint16_t airflow = (requestedVentLevel == 0) ? 0 :
                       (requestedVentLevel == 1) ? 100 :
                       (requestedVentLevel == 2) ? 130 : 195;
    cwlData.tspValues[52] = airflow & 0xFF;
    cwlData.tspValues[53] = (airflow >> 8) & 0xFF;
    cwlData.tspValid[52] = true;
    cwlData.tspValid[53] = true;

    cwlData.tspValues[55] = 100 + (uint8_t)(14 + 3 * sin(t / 120.0f));  // ~14°C outdoor
    cwlData.tspValid[55] = true;
    cwlData.tspValues[56] = 121;  // 21°C indoor
    cwlData.tspValid[56] = true;

    cwlData.tspValues[64] = 150 + (uint8_t)(30 * sin(t / 40.0f));  // Input pressure ~150 Pa
    cwlData.tspValues[65] = 0;
    cwlData.tspValid[64] = true;
    cwlData.tspValid[65] = true;
    cwlData.tspValues[66] = 200 + (uint8_t)(20 * sin(t / 50.0f));  // Output pressure ~200 Pa
    cwlData.tspValues[67] = 0;
    cwlData.tspValid[66] = true;
    cwlData.tspValid[67] = true;

    updateDerivedValues();

    // Handle filter reset
    if (requestedFilterReset) {
        requestedFilterReset = false;
        log("OpenTherm [SIM]: Filter reset acknowledged");
    }
}

// =============================================================================
// REAL MODE — actual OpenTherm communication via DIYLess shield
// =============================================================================
#else

#include <OpenTherm.h>

static OpenTherm* ot = nullptr;
static bool filterResetActive = false;

enum PollState {
    POLL_MASTER_CONFIG,
    POLL_STATUS,
    POLL_SETPOINT,
    POLL_CONFIG,
    POLL_VENTILATION,
    POLL_SUPPLY_TEMP,
    POLL_EXHAUST_TEMP,
    POLL_MASTER_VERSION,
    POLL_SLAVE_VERSION,
    POLL_TSP,
    POLL_FAULT,
    POLL_SUPPLY_OUTLET,
    POLL_EXHAUST_OUTLET,
    POLL_EXHAUST_FAN,
    POLL_SUPPLY_FAN,
    POLL_CYCLE_DONE
};

static PollState pollState = POLL_MASTER_CONFIG;
static unsigned long lastPollTime = 0;
static uint8_t tspIndex = 0;
static bool probeComplete = false;

#define POLL_INTERVAL 1000

static void IRAM_ATTR handleInterrupt() {
    ot->handleInterrupt();
}

void setupOpenTherm() {
    ot = new OpenTherm(appConfig.otInPin, appConfig.otOutPin);
    ot->begin(handleInterrupt);
    log("OpenTherm: Initialized (IN=" + String(appConfig.otInPin) +
        ", OUT=" + String(appConfig.otOutPin) + ")");
    requestedVentLevel = appConfig.ventilationLevel;
    requestedBypassOpen = appConfig.bypassOpen;
}

static bool sendRequest(unsigned long request, unsigned long& response) {
    response = ot->sendRequest(request);
    OpenThermResponseStatus status = ot->getLastResponseStatus();
    if (status == OpenThermResponseStatus::SUCCESS) {
        cwlData.connected = true;
        cwlData.lastResponse = millis();
        return true;
    }
    if (status == OpenThermResponseStatus::TIMEOUT) {
        if (cwlData.connected && millis() - cwlData.lastResponse > 30000) {
            cwlData.connected = false;
            log("OpenTherm: CWL not responding");
        }
    }
    return false;
}

static void processStatus(unsigned long response) {
    uint8_t hiResp = (response >> 8) & 0xFF;
    cwlData.fault = hiResp & 0x01;
    cwlData.ventilationActive = (hiResp >> 1) & 0x01;
    cwlData.coolingActive = (hiResp >> 2) & 0x01;
    cwlData.dhwActive = (hiResp >> 3) & 0x01;
    cwlData.filterDirty = (hiResp >> 4) & 0x01;
    cwlData.diagEvent = (hiResp >> 5) & 0x01;
}

static void processVentilation(unsigned long response) {
    cwlData.relativeVentilation = (response >> 8) & 0xFF;
}

static void processTemperature(uint8_t dataId, unsigned long response) {
    float temp = decodeF88(response & 0xFFFF);
    switch (dataId) {
        case 80: cwlData.supplyInletTemp = temp; break;
        case 81: cwlData.supplyOutletTemp = temp; break;
        case 82: cwlData.exhaustInletTemp = temp; break;
        case 83: cwlData.exhaustOutletTemp = temp; break;
    }
}

static void processFault(unsigned long response) {
    cwlData.faultFlags = (response >> 8) & 0xFF;
    cwlData.oemFaultCode = response & 0xFF;
}

static void processConfig(unsigned long response) {
    cwlData.slaveFlags = (response >> 8) & 0xFF;
    cwlData.slaveMemberId = response & 0xFF;
}

static void processTsp(unsigned long response) {
    uint8_t index = (response >> 8) & 0xFF;
    uint8_t value = response & 0xFF;
    if (index < TSP_MAX_INDEX) {
        cwlData.tspValues[index] = value;
        cwlData.tspValid[index] = true;
        updateDerivedValues();
    }
}

static void processFanSpeed(uint8_t dataId, unsigned long response) {
    uint16_t speed = response & 0xFFFF;
    if (dataId == 84) cwlData.exhaustFanSpeed = speed;
    else if (dataId == 85) cwlData.supplyFanSpeed = speed;
}

static unsigned long buildFrame(OpenThermMessageType type, uint8_t dataId, uint16_t data) {
    unsigned long frame = 0;
    frame |= ((unsigned long)type) << 28;
    frame |= ((unsigned long)dataId) << 16;
    frame |= data;
    if (OpenTherm::parity(frame)) {
        frame |= 0x80000000UL;
    }
    return frame;
}

static unsigned long buildStatusRequest() {
    uint8_t flags = 0x01;
    if (requestedBypassOpen) flags |= 0x02;
    if (filterResetActive) flags |= 0x10;
    return buildFrame(OpenThermMessageType::READ_DATA, 70, (uint16_t)(flags << 8));
}

static unsigned long buildSetpointRequest() {
    uint8_t level = requestedVentLevel;
    if (level > 3) level = 2;
    cwlData.ventilationLevel = level;
    return buildFrame(OpenThermMessageType::WRITE_DATA, 71, (uint16_t)level);
}

void probeAdditionalIds() {
    log("OpenTherm: Probing additional data IDs...");

    struct ProbeEntry {
        uint8_t id;
        bool* supportFlag;
        const char* name;
    };

    ProbeEntry probes[] = {
        {81, &cwlData.supportsId81, "Supply outlet temp"},
        {83, &cwlData.supportsId83, "Exhaust outlet temp"},
        {84, &cwlData.supportsId84, "Exhaust fan speed"},
        {85, &cwlData.supportsId85, "Supply fan speed"},
        {78, &cwlData.supportsId78, "Relative humidity"},
        {79, &cwlData.supportsId79, "CO2 level"},
        {87, &cwlData.supportsId87, "Nominal ventilation"},
        {88, &cwlData.supportsId88, "TSP count"},
    };

    for (auto& probe : probes) {
        unsigned long request = buildFrame(OpenThermMessageType::READ_DATA, probe.id, 0);
        unsigned long response = ot->sendRequest(request);
        OpenThermResponseStatus status = ot->getLastResponseStatus();

        if (status == OpenThermResponseStatus::SUCCESS) {
            OpenThermMessageType msgType = OpenTherm::getMessageType(response);
            if (msgType == OpenThermMessageType::READ_ACK) {
                *probe.supportFlag = true;
                log("  ID " + String(probe.id) + " (" + probe.name + "): supported");
            } else {
                *probe.supportFlag = false;
                log("  ID " + String(probe.id) + " (" + probe.name + "): not supported");
            }
        } else {
            *probe.supportFlag = false;
            log("  ID " + String(probe.id) + " (" + probe.name + "): no response");
        }
        delay(100);
    }

    probeComplete = true;
    log("OpenTherm: Probe complete");
}

void openThermLoop() {
    unsigned long now = millis();
    if (now - lastPollTime < POLL_INTERVAL) return;
    lastPollTime = now;

    if (!probeComplete && cwlData.connected) {
        probeAdditionalIds();
        return;
    }

    unsigned long request, response;

    switch (pollState) {
        case POLL_MASTER_CONFIG:
            request = buildFrame(OpenThermMessageType::WRITE_DATA, 2, 0x0012);
            sendRequest(request, response);
            pollState = POLL_STATUS;
            break;
        case POLL_STATUS:
            request = buildStatusRequest();
            if (sendRequest(request, response)) processStatus(response);
            if (filterResetActive) { filterResetActive = false; log("OpenTherm: Filter reset cleared"); }
            pollState = POLL_SETPOINT;
            break;
        case POLL_SETPOINT:
            request = buildSetpointRequest();
            sendRequest(request, response);
            pollState = POLL_CONFIG;
            break;
        case POLL_CONFIG:
            request = buildFrame(OpenThermMessageType::READ_DATA, 74, 0);
            if (sendRequest(request, response)) processConfig(response);
            pollState = POLL_VENTILATION;
            break;
        case POLL_VENTILATION:
            request = buildFrame(OpenThermMessageType::READ_DATA, 77, 0);
            if (sendRequest(request, response)) processVentilation(response);
            pollState = POLL_SUPPLY_TEMP;
            break;
        case POLL_SUPPLY_TEMP:
            request = buildFrame(OpenThermMessageType::READ_DATA, 80, 0);
            if (sendRequest(request, response)) processTemperature(80, response);
            pollState = POLL_EXHAUST_TEMP;
            break;
        case POLL_EXHAUST_TEMP:
            request = buildFrame(OpenThermMessageType::READ_DATA, 82, 0);
            if (sendRequest(request, response)) processTemperature(82, response);
            pollState = POLL_MASTER_VERSION;
            break;
        case POLL_MASTER_VERSION:
            request = buildFrame(OpenThermMessageType::WRITE_DATA, 126, 0x1202);
            sendRequest(request, response);
            pollState = POLL_SLAVE_VERSION;
            break;
        case POLL_SLAVE_VERSION:
            request = buildFrame(OpenThermMessageType::READ_DATA, 127, 0);
            if (sendRequest(request, response)) {
                cwlData.slaveType = (response >> 8) & 0xFF;
                cwlData.slaveVersion = response & 0xFF;
            }
            pollState = POLL_TSP;
            break;
        case POLL_TSP:
            request = buildFrame(OpenThermMessageType::READ_DATA, 89, (uint16_t)(tspIndex << 8));
            if (sendRequest(request, response)) processTsp(response);
            tspIndex++;
            if (tspIndex >= TSP_MAX_INDEX) tspIndex = 0;
            pollState = POLL_FAULT;
            break;
        case POLL_FAULT:
            request = buildFrame(OpenThermMessageType::READ_DATA, 72, 0);
            if (sendRequest(request, response)) processFault(response);
            pollState = cwlData.supportsId81 ? POLL_SUPPLY_OUTLET :
                        (cwlData.supportsId83 ? POLL_EXHAUST_OUTLET :
                        (cwlData.supportsId84 ? POLL_EXHAUST_FAN :
                        (cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE)));
            break;
        case POLL_SUPPLY_OUTLET:
            request = buildFrame(OpenThermMessageType::READ_DATA, 81, 0);
            if (sendRequest(request, response)) processTemperature(81, response);
            pollState = cwlData.supportsId83 ? POLL_EXHAUST_OUTLET :
                        (cwlData.supportsId84 ? POLL_EXHAUST_FAN :
                        (cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE));
            break;
        case POLL_EXHAUST_OUTLET:
            request = buildFrame(OpenThermMessageType::READ_DATA, 83, 0);
            if (sendRequest(request, response)) processTemperature(83, response);
            pollState = cwlData.supportsId84 ? POLL_EXHAUST_FAN :
                        (cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE);
            break;
        case POLL_EXHAUST_FAN:
            request = buildFrame(OpenThermMessageType::READ_DATA, 84, 0);
            if (sendRequest(request, response)) processFanSpeed(84, response);
            pollState = cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE;
            break;
        case POLL_SUPPLY_FAN:
            request = buildFrame(OpenThermMessageType::READ_DATA, 85, 0);
            if (sendRequest(request, response)) processFanSpeed(85, response);
            pollState = POLL_CYCLE_DONE;
            break;
        case POLL_CYCLE_DONE:
            pollState = POLL_MASTER_CONFIG;
            break;
    }

    if (requestedFilterReset) {
        requestedFilterReset = false;
        filterResetActive = true;
        log("OpenTherm: Filter reset requested");
    }
}

#endif // SIMULATE_OT
