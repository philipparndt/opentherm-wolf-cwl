#include "mqtt_client.h"
#include "config_manager.h"
#include "logging.h"
#include "config.h"
#include "wolf_cwl.h"
#include "ot_master.h"
#include "watchdog.h"
#include "scheduler.h"
#include <WiFi.h>
#include <WiFiClient.h>
#include <ArduinoJson.h>

// Firmware version - defined by build script
#ifndef FIRMWARE_VERSION
#define FIRMWARE_VERSION "dev"
#endif

static WiFiClient netClient;
PubSubClient mqtt(netClient);

static unsigned long lastMqttReconnect = 0;
static unsigned long lastPublish = 0;
static unsigned long lastHealthPublish = 0;
#define MQTT_RETRY_INTERVAL 5000
#define MQTT_PUBLISH_INTERVAL 11000   // Publish after each ~11s polling cycle
#define HEALTH_PUBLISH_INTERVAL 60000 // Health telemetry every 60s

static void mqttCallback(char* topic, byte* payload, unsigned int length);

void setupMqtt() {
    if (!appConfig.mqttEnabled) {
        log("MQTT: Disabled");
        return;
    }

    if (strlen(appConfig.mqttServer) == 0) {
        log("MQTT: No server configured");
        return;
    }

    log("MQTT: Server=" + String(appConfig.mqttServer) +
        " Port=" + String(appConfig.mqttPort) +
        " Topic=" + String(appConfig.mqttTopic) +
        " Auth=" + String(appConfig.mqttAuthEnabled ? "yes" : "no"));

    mqtt.setServer(appConfig.mqttServer, appConfig.mqttPort);
    mqtt.setCallback(mqttCallback);
    mqtt.setBufferSize(1024);
    mqtt.setKeepAlive(60);
    mqtt.setSocketTimeout(10);
    log("MQTT: Configured");
}

void mqttLoop() {
    mqtt.loop();

    // Periodic sensor publishing
    unsigned long now = millis();
    if (mqtt.connected() && cwlData.connected && now - lastPublish > MQTT_PUBLISH_INTERVAL) {
        lastPublish = now;
        publishSensorData();
    }

    // Health telemetry publishing
    if (mqtt.connected() && now - lastHealthPublish > HEALTH_PUBLISH_INTERVAL) {
        lastHealthPublish = now;
        publishHealthData();
    }
}

void mqttReconnect() {
    if (!appConfig.mqttEnabled) return;
    if (mqtt.connected()) return;
    if (strlen(appConfig.mqttServer) == 0) return;
    if (millis() - lastMqttReconnect < MQTT_RETRY_INTERVAL) return;

    lastMqttReconnect = millis();
    log("MQTT: Connecting to " + String(appConfig.mqttServer) + ":" + String(appConfig.mqttPort));

    // Test TCP connectivity first
    if (!netClient.connect(appConfig.mqttServer, appConfig.mqttPort)) {
        log("MQTT: TCP connection failed");
        return;
    }
    netClient.stop();

    String clientId = "wolf-cwl-" + String(random(0xffff), HEX);

    // LWT for bridge state
    String willTopic = String(appConfig.mqttTopic) + "/bridge/state";
    const char* willMessage = "offline";

    bool connected = false;
    if (appConfig.mqttAuthEnabled && strlen(appConfig.mqttUsername) > 0) {
        connected = mqtt.connect(clientId.c_str(), appConfig.mqttUsername, appConfig.mqttPassword,
                                 willTopic.c_str(), 0, true, willMessage);
    } else {
        connected = mqtt.connect(clientId.c_str(), willTopic.c_str(), 0, true, willMessage);
    }

    if (connected) {
        log("MQTT: Connected");

        // Subscribe to command topics
        String setLevel = String(appConfig.mqttTopic) + "/set/level";
        String setBypass = String(appConfig.mqttTopic) + "/set/bypass";
        String setFilter = String(appConfig.mqttTopic) + "/set/filter_reset";

        mqtt.subscribe(setLevel.c_str());
        mqtt.subscribe(setBypass.c_str());
        mqtt.subscribe(setFilter.c_str());
        log("MQTT: Subscribed to command topics");

        publishBridgeInfo();
    } else {
        int state = mqtt.state();
        String errorMsg;
        switch (state) {
            case -4: errorMsg = "TIMEOUT"; break;
            case -3: errorMsg = "CONNECTION_LOST"; break;
            case -2: errorMsg = "CONNECT_FAILED"; break;
            case -1: errorMsg = "DISCONNECTED"; break;
            case 4: errorMsg = "BAD_CREDENTIALS"; break;
            case 5: errorMsg = "UNAUTHORIZED"; break;
            default: errorMsg = "ERROR_" + String(state); break;
        }
        log("MQTT: Failed (" + errorMsg + ")");
    }
}

void publishBridgeInfo() {
    if (!mqtt.connected()) return;

    String baseTopic = String(appConfig.mqttTopic) + "/bridge";

    mqtt.publish((baseTopic + "/state").c_str(), "online", true);
    mqtt.publish((baseTopic + "/version").c_str(), FIRMWARE_VERSION, true);
    mqtt.publish((baseTopic + "/ip").c_str(), WiFi.localIP().toString().c_str(), true);

    log("MQTT: Published bridge info (version=" + String(FIRMWARE_VERSION) + ")");
}

void publishSensorData() {
    if (!mqtt.connected()) return;

    String base = String(appConfig.mqttTopic);

    // Ventilation
    mqtt.publish((base + "/ventilation/level").c_str(), String(cwlData.ventilationLevel).c_str(), true);
    mqtt.publish((base + "/ventilation/level_name").c_str(), getVentilationLevelName(cwlData.ventilationLevel), true);
    mqtt.publish((base + "/ventilation/relative").c_str(), String(cwlData.relativeVentilation).c_str(), true);

    // Temperatures
    mqtt.publish((base + "/temperature/supply_inlet").c_str(), String(cwlData.supplyInletTemp, 1).c_str(), true);
    mqtt.publish((base + "/temperature/exhaust_inlet").c_str(), String(cwlData.exhaustInletTemp, 1).c_str(), true);
    if (cwlData.supportsId81) {
        mqtt.publish((base + "/temperature/supply_outlet").c_str(), String(cwlData.supplyOutletTemp, 1).c_str(), true);
    }
    if (cwlData.supportsId83) {
        mqtt.publish((base + "/temperature/exhaust_outlet").c_str(), String(cwlData.exhaustOutletTemp, 1).c_str(), true);
    }

    // Status flags
    mqtt.publish((base + "/status/fault").c_str(), cwlData.fault ? "1" : "0", true);
    mqtt.publish((base + "/status/filter").c_str(), cwlData.filterDirty ? "1" : "0", true);
    mqtt.publish((base + "/status/bypass").c_str(), cwlData.ventilationActive ? "1" : "0", true);

    // Fan speeds (if supported)
    if (cwlData.supportsId84) {
        mqtt.publish((base + "/fan/exhaust_speed").c_str(), String(cwlData.exhaustFanSpeed).c_str(), true);
    }
    if (cwlData.supportsId85) {
        mqtt.publish((base + "/fan/supply_speed").c_str(), String(cwlData.supplyFanSpeed).c_str(), true);
    }

    // TSP-derived values
    if (cwlData.tspValid[52]) {
        mqtt.publish((base + "/airflow/current_volume").c_str(), String(cwlData.currentVolume).c_str(), true);
    }
    if (cwlData.tspValid[54]) {
        mqtt.publish((base + "/status/bypass_position").c_str(), String(cwlData.bypassStatus).c_str(), true);
    }
    if (cwlData.tspValid[55]) {
        mqtt.publish((base + "/temperature/atmospheric").c_str(), String(cwlData.tempAtmospheric).c_str(), true);
    }
    if (cwlData.tspValid[56]) {
        mqtt.publish((base + "/temperature/indoor").c_str(), String(cwlData.tempIndoor).c_str(), true);
    }
    if (cwlData.tspValid[64]) {
        mqtt.publish((base + "/pressure/input_duct").c_str(), String(cwlData.inputDuctPressure).c_str(), true);
    }
    if (cwlData.tspValid[66]) {
        mqtt.publish((base + "/pressure/output_duct").c_str(), String(cwlData.outputDuctPressure).c_str(), true);
    }
    if (cwlData.tspValid[68]) {
        mqtt.publish((base + "/status/frost").c_str(), String(cwlData.frostStatus).c_str(), true);
    }

    // Schedule state
    mqtt.publish((base + "/schedule/active").c_str(), scheduleActive ? "1" : "0", true);
    mqtt.publish((base + "/schedule/override").c_str(), scheduleOverride ? "1" : "0", true);
    if (scheduleActive) {
        mqtt.publish((base + "/schedule/entry_name").c_str(), getActiveScheduleDescription().c_str(), true);
    }

    // Bypass/summer mode state
    mqtt.publish((base + "/bypass/mode").c_str(), requestedBypassOpen ? "summer" : "winter", true);
    mqtt.publish((base + "/bypass/schedule_active").c_str(), bypassScheduleActive ? "1" : "0", true);
    mqtt.publish((base + "/bypass/override").c_str(), bypassOverride ? "1" : "0", true);
}

void publishHealthData() {
    if (!mqtt.connected()) return;

    String base = String(appConfig.mqttTopic) + "/health";

    mqtt.publish((base + "/uptime").c_str(), String(getUptimeSeconds()).c_str(), true);
    mqtt.publish((base + "/reboot_reason").c_str(), getRebootReasonStr(), true);
    mqtt.publish((base + "/crash_count").c_str(), String(getCrashCount()).c_str(), true);
    mqtt.publish((base + "/free_heap").c_str(), String(ESP.getFreeHeap()).c_str(), true);
    mqtt.publish((base + "/ot_response_age").c_str(), String(getOtResponseAge()).c_str(), true);
}

static void mqttCallback(char* topic, byte* payload, unsigned int length) {
    String topicStr = String(topic);
    String message = "";
    for (unsigned int i = 0; i < length; i++) {
        message += (char)payload[i];
    }

    log("MQTT: Received [" + topicStr + "]: " + message);

    String base = String(appConfig.mqttTopic);

    // Set ventilation level
    if (topicStr == base + "/set/level") {
        int level = message.toInt();
        if (level >= 0 && level <= 3) {
            requestedVentLevel = level;
            appConfig.ventilationLevel = level;
            saveConfig();
            setVentilationManualOverride();
            log("MQTT: Ventilation level set to " + String(level) +
                " (" + getVentilationLevelName(level) + ")");
        } else {
            log("MQTT: Invalid ventilation level: " + message);
        }
        return;
    }

    // Set bypass
    if (topicStr == base + "/set/bypass") {
        bool open = (message == "1" || message == "true" || message == "on");
        requestedBypassOpen = open;
        appConfig.bypassOpen = open;
        saveConfig();
        setBypassManualOverride();
        log("MQTT: Bypass " + String(open ? "open (summer)" : "closed (winter)"));
        return;
    }

    // Filter reset
    if (topicStr == base + "/set/filter_reset") {
        if (message == "1" || message == "true") {
            requestedFilterReset = true;
            log("MQTT: Filter reset triggered");
        }
        return;
    }
}

void publishMqttLog(const String& message) {
    if (!mqtt.connected()) return;
    if (!appConfig.mqttEnabled) return;

    String topic = String(appConfig.mqttTopic) + "/bridge/logs";
    mqtt.publish(topic.c_str(), message.c_str(), false);
}
