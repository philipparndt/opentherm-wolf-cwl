/**
 * Configuration for Wolf CWL OpenTherm Controller
 *
 * Controls a Wolf CWL 300 ventilation unit via OpenTherm protocol
 * using a DIYLess Master OpenTherm Shield.
 *
 * FIRST TIME SETUP:
 * 1. Build and flash the firmware
 * 2. Device starts in AP mode: connect to "WolfCWL-XXXX" WiFi
 * 3. Configure WiFi and MQTT settings via web UI
 * 4. Device restarts and connects to your network
 * 5. Access web UI at http://wolf-cwl.local
 */

#ifndef CONFIG_H
#define CONFIG_H

// =============================================================================
// WiFi Settings
// =============================================================================
// Leave empty for AP mode setup via web UI
// #define WIFI_SSID           ""
// #define WIFI_PASSWORD       ""

// =============================================================================
// MQTT Settings
// =============================================================================
// Configure via web UI
// #define MQTT_SERVER         ""
// #define MQTT_PORT           1883
// #define MQTT_USERNAME       ""
// #define MQTT_PASSWORD       ""
// #define MQTT_TOPIC          "wolf-cwl"

// =============================================================================
// OpenTherm Pin Configuration (DIYLess Thermostat Shield)
// =============================================================================
// Default pins for DIYLess shield, override in platformio.ini if needed
// #define PIN_OT_IN           21
// #define PIN_OT_OUT          22

// =============================================================================
// OLED Display (SH1106 128x64 I2C)
// =============================================================================
// #define PIN_SDA             19
// #define PIN_SCL             23

// =============================================================================
// Rotary Encoder (KY-040)
// =============================================================================
// #define PIN_ENC_CLK         16
// #define PIN_ENC_DT          17
// #define PIN_ENC_SW          5

#endif // CONFIG_H
