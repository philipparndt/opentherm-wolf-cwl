#pragma once

#include <Arduino.h>
#include <Preferences.h>

// Maximum sizes for string fields
#define CFG_MAX_SSID_LEN 33
#define CFG_MAX_PASSWORD_LEN 65
#define CFG_MAX_HOST_LEN 64
#define CFG_MAX_USERNAME_LEN 64
#define CFG_MAX_TOPIC_LEN 128

// Configuration structure
struct AppConfig {
    // WiFi credentials
    char wifiSsid[CFG_MAX_SSID_LEN];
    char wifiPassword[CFG_MAX_PASSWORD_LEN];

    // MQTT settings
    bool mqttEnabled;
    char mqttServer[CFG_MAX_HOST_LEN];
    uint16_t mqttPort;
    char mqttTopic[CFG_MAX_TOPIC_LEN];
    char mqttUsername[CFG_MAX_USERNAME_LEN];
    char mqttPassword[CFG_MAX_PASSWORD_LEN];
    bool mqttAuthEnabled;

    // Web UI authentication
    char webUsername[CFG_MAX_USERNAME_LEN];
    char webPassword[CFG_MAX_PASSWORD_LEN];

    // OpenTherm pins
    uint8_t otInPin;
    uint8_t otOutPin;

    // OLED display pins
    uint8_t sdaPin;
    uint8_t sclPin;

    // Encoder pins
    uint8_t encClkPin;
    uint8_t encDtPin;
    uint8_t encSwPin;

    // Ventilation state (persisted)
    uint8_t ventilationLevel;  // 0-3
    bool bypassOpen;

    // System state
    bool configured;  // false = first run, show AP mode

    // JWT secret for persistent session tokens (32 bytes)
    uint8_t jwtSecret[32];
    bool jwtSecretInitialized;
};

// Global config instance
extern AppConfig appConfig;

// Configuration management functions
void initConfigManager();
void loadConfig();
void saveConfig();
void resetConfig();

// Helper to check if WiFi credentials are set
bool hasWifiCredentials();

// Get config as JSON (for web API, passwords masked)
String getConfigJson(bool maskPasswords = true);

// Update config from JSON (for web API)
bool updateConfigFromJson(const String& json);
