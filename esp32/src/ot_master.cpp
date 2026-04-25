#include "ot_master.h"
#include "config_manager.h"
#include "wolf_cwl.h"
#include "logging.h"
#include <OpenTherm.h>

// OpenTherm library instance (pins set in setup)
static OpenTherm* ot = nullptr;

bool otConnected = false;

// Control state (written by MQTT/encoder, read by polling cycle)
uint8_t requestedVentLevel = 2;   // Normal
bool requestedBypassOpen = false;
bool requestedFilterReset = false;
static bool filterResetActive = false;

// Polling cycle state machine
enum PollState {
    POLL_MASTER_CONFIG,      // ID 2: Write master config
    POLL_STATUS,             // ID 70: Read status with control flags
    POLL_SETPOINT,           // ID 71: Write ventilation level
    POLL_CONFIG,             // ID 74: Read config/member-id
    POLL_VENTILATION,        // ID 77: Read relative ventilation %
    POLL_SUPPLY_TEMP,        // ID 80: Read supply inlet temperature
    POLL_EXHAUST_TEMP,       // ID 82: Read exhaust inlet temperature
    POLL_MASTER_VERSION,     // ID 126: Write master product version
    POLL_SLAVE_VERSION,      // ID 127: Read slave product version
    POLL_TSP,                // ID 89: Read TSP index/value
    POLL_FAULT,              // ID 72: Read fault flags
    POLL_SUPPLY_OUTLET,      // ID 81
    POLL_EXHAUST_OUTLET,     // ID 83
    POLL_EXHAUST_FAN,        // ID 84
    POLL_SUPPLY_FAN,         // ID 85
    POLL_CYCLE_DONE
};

static PollState pollState = POLL_MASTER_CONFIG;
static unsigned long lastPollTime = 0;
static uint8_t tspIndex = 0;
static bool probeComplete = false;

#define POLL_INTERVAL 1000  // 1 second between requests

// =============================================================================
// OpenTherm interrupt handler
// =============================================================================
static void IRAM_ATTR handleInterrupt() {
    ot->handleInterrupt();
}

void setupOpenTherm() {
    ot = new OpenTherm(appConfig.otInPin, appConfig.otOutPin);
    ot->begin(handleInterrupt);
    log("OpenTherm: Initialized (IN=" + String(appConfig.otInPin) +
        ", OUT=" + String(appConfig.otOutPin) + ")");

    // Initialize control state from config
    requestedVentLevel = appConfig.ventilationLevel;
    requestedBypassOpen = appConfig.bypassOpen;
}

// =============================================================================
// Send a request and process the response
// =============================================================================
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

// =============================================================================
// Process individual data IDs
// =============================================================================
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

// =============================================================================
// Build request frames
// The OpenTherm library's buildRequest takes OpenThermMessageID enum values,
// but V/H data IDs (70+) are not in the enum. We build frames manually.
// =============================================================================
static unsigned long buildFrame(OpenThermMessageType type, uint8_t dataId, uint16_t data) {
    unsigned long frame = 0;
    frame |= ((unsigned long)type) << 28;
    frame |= ((unsigned long)dataId) << 16;
    frame |= data;
    // Set parity
    if (OpenTherm::parity(frame)) {
        frame |= 0x80000000UL;
    }
    return frame;
}

static unsigned long buildStatusRequest() {
    uint8_t flags = 0x01;  // bit 0: ch/ventilation enable
    if (requestedBypassOpen) flags |= 0x02;  // bit 1: bypass open
    if (filterResetActive) flags |= 0x10;    // bit 4: filter reset
    return buildFrame(OpenThermMessageType::READ_DATA, 70, (uint16_t)(flags << 8));
}

static unsigned long buildSetpointRequest() {
    uint8_t level = requestedVentLevel;
    if (level > 3) level = 2;
    cwlData.ventilationLevel = level;
    return buildFrame(OpenThermMessageType::WRITE_DATA, 71, (uint16_t)level);
}

// =============================================================================
// Probe additional V/H data IDs
// =============================================================================
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

// =============================================================================
// Main polling loop
// =============================================================================
void openThermLoop() {
    unsigned long now = millis();
    if (now - lastPollTime < POLL_INTERVAL) return;
    lastPollTime = now;

    // Run probe once on first successful communication
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
            if (sendRequest(request, response)) {
                processStatus(response);
            }
            if (filterResetActive) {
                filterResetActive = false;
                log("OpenTherm: Filter reset cleared");
            }
            pollState = POLL_SETPOINT;
            break;

        case POLL_SETPOINT:
            request = buildSetpointRequest();
            sendRequest(request, response);
            pollState = POLL_CONFIG;
            break;

        case POLL_CONFIG:
            request = buildFrame(OpenThermMessageType::READ_DATA, 74, 0);
            if (sendRequest(request, response)) {
                processConfig(response);
            }
            pollState = POLL_VENTILATION;
            break;

        case POLL_VENTILATION:
            request = buildFrame(OpenThermMessageType::READ_DATA, 77, 0);
            if (sendRequest(request, response)) {
                processVentilation(response);
            }
            pollState = POLL_SUPPLY_TEMP;
            break;

        case POLL_SUPPLY_TEMP:
            request = buildFrame(OpenThermMessageType::READ_DATA, 80, 0);
            if (sendRequest(request, response)) {
                processTemperature(80, response);
            }
            pollState = POLL_EXHAUST_TEMP;
            break;

        case POLL_EXHAUST_TEMP:
            request = buildFrame(OpenThermMessageType::READ_DATA, 82, 0);
            if (sendRequest(request, response)) {
                processTemperature(82, response);
            }
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
            if (sendRequest(request, response)) {
                processTsp(response);
            }
            tspIndex++;
            if (tspIndex >= TSP_MAX_INDEX) tspIndex = 0;
            pollState = POLL_FAULT;
            break;

        case POLL_FAULT:
            request = buildFrame(OpenThermMessageType::READ_DATA, 72, 0);
            if (sendRequest(request, response)) {
                processFault(response);
            }
            pollState = cwlData.supportsId81 ? POLL_SUPPLY_OUTLET :
                        (cwlData.supportsId83 ? POLL_EXHAUST_OUTLET :
                        (cwlData.supportsId84 ? POLL_EXHAUST_FAN :
                        (cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE)));
            break;

        case POLL_SUPPLY_OUTLET:
            request = buildFrame(OpenThermMessageType::READ_DATA, 81, 0);
            if (sendRequest(request, response)) {
                processTemperature(81, response);
            }
            pollState = cwlData.supportsId83 ? POLL_EXHAUST_OUTLET :
                        (cwlData.supportsId84 ? POLL_EXHAUST_FAN :
                        (cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE));
            break;

        case POLL_EXHAUST_OUTLET:
            request = buildFrame(OpenThermMessageType::READ_DATA, 83, 0);
            if (sendRequest(request, response)) {
                processTemperature(83, response);
            }
            pollState = cwlData.supportsId84 ? POLL_EXHAUST_FAN :
                        (cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE);
            break;

        case POLL_EXHAUST_FAN:
            request = buildFrame(OpenThermMessageType::READ_DATA, 84, 0);
            if (sendRequest(request, response)) {
                processFanSpeed(84, response);
            }
            pollState = cwlData.supportsId85 ? POLL_SUPPLY_FAN : POLL_CYCLE_DONE;
            break;

        case POLL_SUPPLY_FAN:
            request = buildFrame(OpenThermMessageType::READ_DATA, 85, 0);
            if (sendRequest(request, response)) {
                processFanSpeed(85, response);
            }
            pollState = POLL_CYCLE_DONE;
            break;

        case POLL_CYCLE_DONE:
            pollState = POLL_MASTER_CONFIG;
            break;
    }

    // Handle filter reset request
    if (requestedFilterReset) {
        requestedFilterReset = false;
        filterResetActive = true;
        log("OpenTherm: Filter reset requested");
    }
}
