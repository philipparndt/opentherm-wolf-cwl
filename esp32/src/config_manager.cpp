#include "config_manager.h"
#include "logging.h"
#include "config.h"
#include <ArduinoJson.h>

// Global config instance
AppConfig appConfig;

// Preferences namespace
static Preferences prefs;
static const char* PREFS_NAMESPACE = "wolfcwl";

void initConfigManager() {
    memset(&appConfig, 0, sizeof(AppConfig));

    appConfig.mqttPort = 1883;
    appConfig.mqttEnabled = false;
    appConfig.configured = false;

    // Default web credentials
    strncpy(appConfig.webUsername, "admin", CFG_MAX_USERNAME_LEN - 1);
    strncpy(appConfig.webPassword, "admin", CFG_MAX_PASSWORD_LEN - 1);

    // Default MQTT topic
    strncpy(appConfig.mqttTopic, "wolf-cwl", CFG_MAX_TOPIC_LEN - 1);

    // Default pin assignments from build flags
    appConfig.otInPin = PIN_OT_IN;
    appConfig.otOutPin = PIN_OT_OUT;
    appConfig.sdaPin = PIN_SDA;
    appConfig.sclPin = PIN_SCL;
    appConfig.encClkPin = PIN_ENC_CLK;
    appConfig.encDtPin = PIN_ENC_DT;
    appConfig.encSwPin = PIN_ENC_SW;

    // Default ventilation state
    appConfig.ventilationLevel = 2;  // Normal
    appConfig.bypassOpen = false;
}

void loadConfig() {
    initConfigManager();

    prefs.begin(PREFS_NAMESPACE, true);  // Read-only mode

    appConfig.configured = prefs.getBool("configured", false);

    if (appConfig.configured) {
        log("Config: Loading from NVS...");

        // WiFi
        String ssid = prefs.getString("wifi_ssid", "");
        String pass = prefs.getString("wifi_pass", "");
        strncpy(appConfig.wifiSsid, ssid.c_str(), CFG_MAX_SSID_LEN - 1);
        strncpy(appConfig.wifiPassword, pass.c_str(), CFG_MAX_PASSWORD_LEN - 1);

        // MQTT
        appConfig.mqttEnabled = prefs.getBool("mqtt_enabled", false);
        String mqttSrv = prefs.getString("mqtt_server", "");
        String mqttTopic = prefs.getString("mqtt_topic", "wolf-cwl");
        String mqttUser = prefs.getString("mqtt_user", "");
        String mqttPass = prefs.getString("mqtt_pass", "");
        strncpy(appConfig.mqttServer, mqttSrv.c_str(), CFG_MAX_HOST_LEN - 1);
        strncpy(appConfig.mqttTopic, mqttTopic.c_str(), CFG_MAX_TOPIC_LEN - 1);
        strncpy(appConfig.mqttUsername, mqttUser.c_str(), CFG_MAX_USERNAME_LEN - 1);
        strncpy(appConfig.mqttPassword, mqttPass.c_str(), CFG_MAX_PASSWORD_LEN - 1);
        appConfig.mqttPort = prefs.getInt("mqtt_port", 1883);
        appConfig.mqttAuthEnabled = prefs.getBool("mqtt_auth", false);

        // Web UI auth
        String webUser = prefs.getString("web_user", "admin");
        String webPass = prefs.getString("web_pass", "admin");
        strncpy(appConfig.webUsername, webUser.c_str(), CFG_MAX_USERNAME_LEN - 1);
        strncpy(appConfig.webPassword, webPass.c_str(), CFG_MAX_PASSWORD_LEN - 1);

        // Pin configuration
        appConfig.otInPin = prefs.getInt("ot_in_pin", PIN_OT_IN);
        appConfig.otOutPin = prefs.getInt("ot_out_pin", PIN_OT_OUT);
        appConfig.sdaPin = prefs.getInt("sda_pin", PIN_SDA);
        appConfig.sclPin = prefs.getInt("scl_pin", PIN_SCL);
        appConfig.encClkPin = prefs.getInt("enc_clk_pin", PIN_ENC_CLK);
        appConfig.encDtPin = prefs.getInt("enc_dt_pin", PIN_ENC_DT);
        appConfig.encSwPin = prefs.getInt("enc_sw_pin", PIN_ENC_SW);

        // Ventilation state
        appConfig.ventilationLevel = prefs.getInt("vent_level", 2);
        appConfig.bypassOpen = prefs.getBool("bypass_open", false);

        // JWT secret
        appConfig.jwtSecretInitialized = prefs.getBool("jwt_init", false);
        if (appConfig.jwtSecretInitialized) {
            prefs.getBytes("jwt_secret", appConfig.jwtSecret, 32);
        }

        log("Config: Loaded from NVS");
    } else {
        log("Config: First run, using defaults...");

        // WiFi (if defined at compile time)
        #ifdef WIFI_SSID
            strncpy(appConfig.wifiSsid, WIFI_SSID, CFG_MAX_SSID_LEN - 1);
        #endif
        #ifdef WIFI_PASSWORD
            strncpy(appConfig.wifiPassword, WIFI_PASSWORD, CFG_MAX_PASSWORD_LEN - 1);
        #endif

        // MQTT
        #ifdef MQTT_SERVER
            strncpy(appConfig.mqttServer, MQTT_SERVER, CFG_MAX_HOST_LEN - 1);
            appConfig.mqttEnabled = true;
        #endif
        #ifdef MQTT_PORT
            appConfig.mqttPort = MQTT_PORT;
        #endif
        #ifdef MQTT_TOPIC
            strncpy(appConfig.mqttTopic, MQTT_TOPIC, CFG_MAX_TOPIC_LEN - 1);
        #endif
        #ifdef MQTT_USERNAME
            strncpy(appConfig.mqttUsername, MQTT_USERNAME, CFG_MAX_USERNAME_LEN - 1);
            strncpy(appConfig.mqttPassword, MQTT_PASSWORD, CFG_MAX_PASSWORD_LEN - 1);
            appConfig.mqttAuthEnabled = true;
        #endif

        // If we have WiFi credentials from config.h, mark as configured
        if (hasWifiCredentials()) {
            appConfig.configured = true;
            log("Config: Migrated from config.h, saving to NVS...");
            prefs.end();
            saveConfig();
            return;
        }

        log("Config: No valid config found, AP mode needed");
    }

    prefs.end();
}

void saveConfig() {
    prefs.begin(PREFS_NAMESPACE, false);  // Read-write mode

    // WiFi
    prefs.putString("wifi_ssid", appConfig.wifiSsid);
    prefs.putString("wifi_pass", appConfig.wifiPassword);

    // MQTT
    prefs.putBool("mqtt_enabled", appConfig.mqttEnabled);
    prefs.putString("mqtt_server", appConfig.mqttServer);
    prefs.putInt("mqtt_port", appConfig.mqttPort);
    prefs.putString("mqtt_topic", appConfig.mqttTopic);
    prefs.putString("mqtt_user", appConfig.mqttUsername);
    prefs.putString("mqtt_pass", appConfig.mqttPassword);
    prefs.putBool("mqtt_auth", appConfig.mqttAuthEnabled);

    // Web UI auth
    prefs.putString("web_user", appConfig.webUsername);
    prefs.putString("web_pass", appConfig.webPassword);

    // Pin configuration
    prefs.putInt("ot_in_pin", appConfig.otInPin);
    prefs.putInt("ot_out_pin", appConfig.otOutPin);
    prefs.putInt("sda_pin", appConfig.sdaPin);
    prefs.putInt("scl_pin", appConfig.sclPin);
    prefs.putInt("enc_clk_pin", appConfig.encClkPin);
    prefs.putInt("enc_dt_pin", appConfig.encDtPin);
    prefs.putInt("enc_sw_pin", appConfig.encSwPin);

    // Ventilation state
    prefs.putInt("vent_level", appConfig.ventilationLevel);
    prefs.putBool("bypass_open", appConfig.bypassOpen);

    // Mark as configured
    prefs.putBool("configured", appConfig.configured);

    // JWT secret
    if (appConfig.jwtSecretInitialized) {
        prefs.putBool("jwt_init", true);
        prefs.putBytes("jwt_secret", appConfig.jwtSecret, 32);
    }

    prefs.end();
    log("Config: Saved to NVS");
}

void resetConfig() {
    prefs.begin(PREFS_NAMESPACE, false);
    prefs.clear();
    prefs.end();

    initConfigManager();
    log("Config: Reset to defaults");
}

bool hasWifiCredentials() {
    return strlen(appConfig.wifiSsid) > 0 && strlen(appConfig.wifiPassword) > 0;
}

String getConfigJson(bool maskPasswords) {
    JsonDocument doc;

    // Network
    doc["network"]["wifiSsid"] = appConfig.wifiSsid;
    doc["network"]["wifiPassword"] = maskPasswords ? "********" : appConfig.wifiPassword;

    // MQTT
    doc["mqtt"]["enabled"] = appConfig.mqttEnabled;
    doc["mqtt"]["server"] = appConfig.mqttServer;
    doc["mqtt"]["port"] = appConfig.mqttPort;
    doc["mqtt"]["topic"] = appConfig.mqttTopic;
    doc["mqtt"]["authEnabled"] = appConfig.mqttAuthEnabled;
    doc["mqtt"]["username"] = appConfig.mqttUsername;
    doc["mqtt"]["password"] = maskPasswords ? "********" : appConfig.mqttPassword;

    // Web UI auth
    doc["web"]["username"] = appConfig.webUsername;
    doc["web"]["password"] = maskPasswords ? "********" : appConfig.webPassword;

    // Pins
    doc["pins"]["otIn"] = appConfig.otInPin;
    doc["pins"]["otOut"] = appConfig.otOutPin;
    doc["pins"]["sda"] = appConfig.sdaPin;
    doc["pins"]["scl"] = appConfig.sclPin;
    doc["pins"]["encClk"] = appConfig.encClkPin;
    doc["pins"]["encDt"] = appConfig.encDtPin;
    doc["pins"]["encSw"] = appConfig.encSwPin;

    // State
    doc["configured"] = appConfig.configured;

    String output;
    serializeJson(doc, output);
    return output;
}

bool updateConfigFromJson(const String& json) {
    JsonDocument doc;
    DeserializationError error = deserializeJson(doc, json);

    if (error) {
        log("Config: JSON parse error: " + String(error.c_str()));
        return false;
    }

    // Network
    if (doc["network"]["wifiSsid"].is<const char*>()) {
        strncpy(appConfig.wifiSsid, doc["network"]["wifiSsid"], CFG_MAX_SSID_LEN - 1);
    }
    if (doc["network"]["wifiPassword"].is<const char*>()) {
        const char* pass = doc["network"]["wifiPassword"];
        if (strcmp(pass, "********") != 0) {
            strncpy(appConfig.wifiPassword, pass, CFG_MAX_PASSWORD_LEN - 1);
        }
    }

    // MQTT
    if (doc["mqtt"]["enabled"].is<bool>()) {
        appConfig.mqttEnabled = doc["mqtt"]["enabled"];
    }
    if (doc["mqtt"]["server"].is<const char*>()) {
        strncpy(appConfig.mqttServer, doc["mqtt"]["server"], CFG_MAX_HOST_LEN - 1);
    }
    if (doc["mqtt"]["port"].is<int>()) {
        appConfig.mqttPort = doc["mqtt"]["port"];
    }
    if (doc["mqtt"]["topic"].is<const char*>()) {
        strncpy(appConfig.mqttTopic, doc["mqtt"]["topic"], CFG_MAX_TOPIC_LEN - 1);
    }
    if (doc["mqtt"]["authEnabled"].is<bool>()) {
        appConfig.mqttAuthEnabled = doc["mqtt"]["authEnabled"];
    }
    if (doc["mqtt"]["username"].is<const char*>()) {
        strncpy(appConfig.mqttUsername, doc["mqtt"]["username"], CFG_MAX_USERNAME_LEN - 1);
    }
    if (doc["mqtt"]["password"].is<const char*>()) {
        const char* pass = doc["mqtt"]["password"];
        if (strcmp(pass, "********") != 0) {
            strncpy(appConfig.mqttPassword, pass, CFG_MAX_PASSWORD_LEN - 1);
        }
    }

    // Web UI auth
    if (doc["web"]["username"].is<const char*>()) {
        strncpy(appConfig.webUsername, doc["web"]["username"], CFG_MAX_USERNAME_LEN - 1);
    }
    if (doc["web"]["password"].is<const char*>()) {
        const char* pass = doc["web"]["password"];
        if (strcmp(pass, "********") != 0) {
            strncpy(appConfig.webPassword, pass, CFG_MAX_PASSWORD_LEN - 1);
        }
    }

    // Pins
    if (doc["pins"]["otIn"].is<int>()) appConfig.otInPin = doc["pins"]["otIn"];
    if (doc["pins"]["otOut"].is<int>()) appConfig.otOutPin = doc["pins"]["otOut"];
    if (doc["pins"]["sda"].is<int>()) appConfig.sdaPin = doc["pins"]["sda"];
    if (doc["pins"]["scl"].is<int>()) appConfig.sclPin = doc["pins"]["scl"];
    if (doc["pins"]["encClk"].is<int>()) appConfig.encClkPin = doc["pins"]["encClk"];
    if (doc["pins"]["encDt"].is<int>()) appConfig.encDtPin = doc["pins"]["encDt"];
    if (doc["pins"]["encSw"].is<int>()) appConfig.encSwPin = doc["pins"]["encSw"];

    // Mark as configured if we have WiFi
    if (hasWifiCredentials()) {
        appConfig.configured = true;
    }

    saveConfig();
    return true;
}
