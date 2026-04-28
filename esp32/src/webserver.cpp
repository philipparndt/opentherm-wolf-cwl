#include "webserver.h"
#include "config_manager.h"
#include "wolf_cwl.h"
#include "ot_master.h"
#include "mqtt_client.h"
#include "logging.h"
#include "ap_mode.h"
#include "scheduler.h"
#include "display.h"
#include <ESPAsyncWebServer.h>
#include <LittleFS.h>
#include <ArduinoJson.h>
#include <ESPmDNS.h>
#include <Update.h>
#include <esp_random.h>

// Firmware version
#ifndef FIRMWARE_VERSION
#define FIRMWARE_VERSION "dev"
#endif

static AsyncWebServer server(80);
static AsyncWebSocket ws("/ws");

// JWT secret management
static void ensureJwtSecret() {
    if (!appConfig.jwtSecretInitialized) {
        for (int i = 0; i < 32; i++) {
            appConfig.jwtSecret[i] = esp_random() & 0xFF;
        }
        appConfig.jwtSecretInitialized = true;
        saveConfig();
        log("WebServer: Generated JWT secret");
    }
}

// Simple JWT validation (check cookie)
static bool isAuthenticated(AsyncWebServerRequest* request) {
    if (apModeActive) return true;  // AP mode bypasses auth

    if (!request->hasHeader("Cookie")) return false;
    String cookie = request->header("Cookie");
    return cookie.indexOf("auth_token=") >= 0;
}

// =============================================================================
// Broadcast log functions (called from logging.h)
// =============================================================================
void broadcastLog(const String& timestamp, const String& message) {
    ws.textAll(timestamp + " " + message);
    publishMqttLog(timestamp + " " + message);
}

void broadcastLogLocal(const String& timestamp, const String& message) {
    ws.textAll(timestamp + " " + message);
}

// =============================================================================
// API Endpoints
// =============================================================================

static void setupApiEndpoints() {
    // Login handler (shared by both paths)
    auto loginHandler = [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        JsonDocument doc;
        deserializeJson(doc, data, len);
        String username = doc["username"] | "";
        String password = doc["password"] | "";

        if (username == appConfig.webUsername && password == appConfig.webPassword) {
            AsyncWebServerResponse* response = request->beginResponse(200, "application/json", "{\"success\":true}");
            response->addHeader("Set-Cookie", "auth_token=valid; Path=/; HttpOnly; Max-Age=86400");
            request->send(response);
        } else {
            request->send(401, "application/json", "{\"error\":\"Invalid credentials\"}");
        }
    };
    auto loginNoop = [](AsyncWebServerRequest* request) {};

    // Login (two paths: /api/login for web UI, /api/auth/login for OTA script)
    server.on("/api/login", HTTP_POST, loginNoop, nullptr, loginHandler);
    server.on("/api/auth/login", HTTP_POST, loginNoop, nullptr, loginHandler);

    // Get config
    server.on("/api/config", HTTP_GET, [](AsyncWebServerRequest* request) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        request->send(200, "application/json", getConfigJson(true));
    });

    // Update config
    server.on("/api/config", HTTP_POST, [](AsyncWebServerRequest* request) {},
              nullptr,
              [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        String json = String((char*)data, len);
        if (updateConfigFromJson(json)) {
            request->send(200, "application/json", "{\"success\":true}");
        } else {
            request->send(400, "application/json", "{\"error\":\"Invalid config\"}");
        }
    });

    // Get current status
    server.on("/api/status", HTTP_GET, [](AsyncWebServerRequest* request) {
        if (!isAuthenticated(request)) { request->send(401); return; }

        JsonDocument doc;
        doc["ventilation"]["level"] = cwlData.ventilationLevel;
        doc["ventilation"]["levelName"] = getVentilationLevelName(cwlData.ventilationLevel);
        doc["ventilation"]["requestedLevel"] = requestedVentLevel;
        doc["ventilation"]["relative"] = cwlData.relativeVentilation;
        doc["ventilation"]["scheduleActive"] = scheduleActive;
        doc["ventilation"]["override"] = scheduleOverride;
        doc["temperature"]["supplyInlet"] = cwlData.supplyInletTemp;
        doc["temperature"]["exhaustInlet"] = cwlData.exhaustInletTemp;
        doc["status"]["fault"] = cwlData.fault;
        doc["status"]["filter"] = cwlData.filterDirty;
        doc["status"]["bypass"] = cwlData.ventilationActive;
        doc["status"]["connected"] = cwlData.connected;
        doc["system"]["uptime"] = millis() / 1000;
        doc["system"]["freeHeap"] = ESP.getFreeHeap();
        doc["system"]["version"] = FIRMWARE_VERSION;
        doc["system"]["mqttConnected"] = mqtt.connected();
        doc["system"]["wifiRssi"] = WiFi.RSSI();
        #ifdef SIMULATE_OT
        doc["system"]["simulated"] = true;
        #else
        doc["system"]["simulated"] = false;
        #endif
        doc["timedOff"]["active"] = timedOffActive;
        doc["timedOff"]["remainingMinutes"] = getTimedOffRemainingMinutes();

        // Airflow rates from TSP registers (m³/h per level)
        uint16_t reducedFlow = (cwlData.tspValid[0] && cwlData.tspValid[1])
            ? (cwlData.tspValues[0] | (cwlData.tspValues[1] << 8)) : 100;
        uint16_t normalFlow = (cwlData.tspValid[2] && cwlData.tspValid[3])
            ? (cwlData.tspValues[2] | (cwlData.tspValues[3] << 8)) : 130;
        uint16_t partyFlow = (cwlData.tspValid[4] && cwlData.tspValid[5])
            ? (cwlData.tspValues[4] | (cwlData.tspValues[5] << 8)) : 195;
        doc["airflow"]["reduced"] = reducedFlow;
        doc["airflow"]["normal"] = normalFlow;
        doc["airflow"]["party"] = partyFlow;

        String output;
        serializeJson(doc, output);
        request->send(200, "application/json", output);
    });

    // Cancel timed off
    server.on("/api/off_timer/cancel", HTTP_POST, [](AsyncWebServerRequest* request) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        cancelTimedOff();
        request->send(200, "application/json", "{\"success\":true}");
    });

    // Set ventilation level
    server.on("/api/ventilation/level", HTTP_POST, [](AsyncWebServerRequest* request) {},
              nullptr,
              [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        JsonDocument doc;
        deserializeJson(doc, data, len);
        int level = doc["level"] | -1;
        if (level >= 0 && level <= 3) {
            requestedVentLevel = level;
            appConfig.ventilationLevel = level;
            setVentilationManualOverride();
            request->send(200, "application/json", "{\"success\":true}");
        } else {
            request->send(400, "application/json", "{\"error\":\"Invalid level (0-3)\"}");
        }
    });

    // Clear ventilation manual override — resume schedule
    server.on("/api/ventilation/resume", HTTP_POST, [](AsyncWebServerRequest* request) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        clearVentilationOverride();
        request->send(200, "application/json", "{\"success\":true}");
    });

    // Virtual encoder (for testing without physical encoder)
    server.on("/api/encoder", HTTP_POST, [](AsyncWebServerRequest* request) {},
              nullptr,
              [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        JsonDocument doc;
        deserializeJson(doc, data, len);
        String action = doc["action"] | "";

        // Wake display first — if it was in standby, consume this input
        if (displayWake()) {
            request->send(200, "application/json", "{\"success\":true}");
            return;
        }

        if (action == "left") {
            if (editMode) adjustEditValue(-1);
            else prevPage();
        } else if (action == "right") {
            if (editMode) adjustEditValue(1);
            else nextPage();
        } else if (action == "press") {
            if (editMode) exitEditMode(true);
            else enterEditMode();
        } else {
            request->send(400, "application/json", "{\"error\":\"Invalid action (left/right/press)\"}");
            return;
        }
        request->send(200, "application/json", "{\"success\":true}");
    });

    // Get ventilation schedules
    server.on("/api/schedules", HTTP_GET, [](AsyncWebServerRequest* request) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        request->send(200, "application/json", getSchedulesJson());
    });

    // Update ventilation schedules
    server.on("/api/schedules", HTTP_POST, [](AsyncWebServerRequest* request) {},
              nullptr,
              [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        String json = String((char*)data, len);
        if (updateSchedulesFromJson(json)) {
            request->send(200, "application/json", "{\"success\":true}");
        } else {
            request->send(400, "application/json", "{\"error\":\"Invalid schedules\"}");
        }
    });

    // Get bypass schedule
    server.on("/api/bypass-schedule", HTTP_GET, [](AsyncWebServerRequest* request) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        request->send(200, "application/json", getBypassScheduleJson());
    });

    // Update bypass schedule
    server.on("/api/bypass-schedule", HTTP_POST, [](AsyncWebServerRequest* request) {},
              nullptr,
              [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        if (!isAuthenticated(request)) { request->send(401); return; }
        String json = String((char*)data, len);
        if (updateBypassScheduleFromJson(json)) {
            request->send(200, "application/json", "{\"success\":true}");
        } else {
            request->send(400, "application/json", "{\"error\":\"Invalid bypass schedule\"}");
        }
    });

    // OTA firmware upload
    server.on("/api/ota/upload", HTTP_POST,
              [](AsyncWebServerRequest* request) {
                  if (Update.hasError()) {
                      request->send(500, "application/json", "{\"error\":\"Update failed\"}");
                  } else {
                      request->send(200, "application/json", "{\"success\":true,\"message\":\"Rebooting...\"}");
                      delay(500);
                      ESP.restart();
                  }
              },
              [](AsyncWebServerRequest* request, const String& filename, size_t index, uint8_t* data, size_t len, bool final) {
                  if (!isAuthenticated(request)) return;
                  if (!index) {
                      log("OTA: Start (" + filename + ")");
                      Update.begin(UPDATE_SIZE_UNKNOWN);
                  }
                  Update.write(data, len);
                  if (final) {
                      if (Update.end(true)) {
                          log("OTA: Success (" + String(index + len) + " bytes)");
                      } else {
                          log("OTA: Failed");
                      }
                  }
              });

    // OTA filesystem upload (LittleFS)
    server.on("/api/ota/filesystem", HTTP_POST,
              [](AsyncWebServerRequest* request) {
                  if (Update.hasError()) {
                      request->send(500, "application/json", "{\"error\":\"Filesystem update failed\"}");
                  } else {
                      request->send(200, "application/json", "{\"success\":true,\"message\":\"Rebooting...\"}");
                      delay(500);
                      ESP.restart();
                  }
              },
              [](AsyncWebServerRequest* request, const String& filename, size_t index, uint8_t* data, size_t len, bool final) {
                  if (!isAuthenticated(request)) return;
                  if (!index) {
                      log("OTA: Filesystem start (" + filename + ")");
                      Update.begin(UPDATE_SIZE_UNKNOWN, U_SPIFFS);
                  }
                  Update.write(data, len);
                  if (final) {
                      if (Update.end(true)) {
                          log("OTA: Filesystem success (" + String(index + len) + " bytes)");
                      } else {
                          log("OTA: Filesystem failed");
                      }
                  }
              });

    // WiFi setup (for AP mode)
    server.on("/api/wifi/setup", HTTP_POST, [](AsyncWebServerRequest* request) {},
              nullptr,
              [](AsyncWebServerRequest* request, uint8_t* data, size_t len, size_t index, size_t total) {
        JsonDocument doc;
        deserializeJson(doc, data, len);

        String ssid = doc["ssid"] | "";
        String pass = doc["password"] | "";

        if (ssid.length() > 0) {
            strncpy(appConfig.wifiSsid, ssid.c_str(), CFG_MAX_SSID_LEN - 1);
            strncpy(appConfig.wifiPassword, pass.c_str(), CFG_MAX_PASSWORD_LEN - 1);

            // Save MQTT if provided
            if (doc["mqtt"]["server"].is<const char*>()) {
                strncpy(appConfig.mqttServer, doc["mqtt"]["server"], CFG_MAX_HOST_LEN - 1);
                appConfig.mqttEnabled = true;
                if (doc["mqtt"]["port"].is<int>()) appConfig.mqttPort = doc["mqtt"]["port"];
                if (doc["mqtt"]["topic"].is<const char*>()) strncpy(appConfig.mqttTopic, doc["mqtt"]["topic"], CFG_MAX_TOPIC_LEN - 1);
                if (doc["mqtt"]["username"].is<const char*>()) {
                    strncpy(appConfig.mqttUsername, doc["mqtt"]["username"], CFG_MAX_USERNAME_LEN - 1);
                    strncpy(appConfig.mqttPassword, doc["mqtt"]["password"] | "", CFG_MAX_PASSWORD_LEN - 1);
                    appConfig.mqttAuthEnabled = true;
                }
            }

            appConfig.configured = true;
            saveConfig();
            request->send(200, "application/json", "{\"success\":true,\"message\":\"Rebooting...\"}");
            delay(1000);
            ESP.restart();
        } else {
            request->send(400, "application/json", "{\"error\":\"SSID required\"}");
        }
    });
}

// =============================================================================
// Setup
// =============================================================================

void setupWebServer() {
    ensureJwtSecret();

    // Mount LittleFS
    if (!LittleFS.begin()) {
        log("WebServer: LittleFS mount failed");
    }

    // mDNS
    if (MDNS.begin("wolf-cwl")) {
        log("WebServer: mDNS started (wolf-cwl.local)");
        MDNS.addService("http", "tcp", 80);
    }

    // WebSocket
    ws.onEvent([](AsyncWebSocket* server, AsyncWebSocketClient* client,
                  AwsEventType type, void* arg, uint8_t* data, size_t len) {
        if (type == WS_EVT_CONNECT) {
            logDebug("WS: Client connected #" + String(client->id()));
        } else if (type == WS_EVT_DISCONNECT) {
            logDebug("WS: Client disconnected #" + String(client->id()));
        }
    });
    server.addHandler(&ws);

    // API endpoints
    setupApiEndpoints();

    // Serve static files from LittleFS
    server.serveStatic("/", LittleFS, "/").setDefaultFile("index.html");

    // 404 handler
    server.onNotFound([](AsyncWebServerRequest* request) {
        if (request->url().startsWith("/api/")) {
            request->send(404, "application/json", "{\"error\":\"Not found\"}");
        } else {
            // SPA fallback
            request->send(LittleFS, "/index.html", "text/html");
        }
    });

    server.begin();
    log("WebServer: Started on port 80");
}

void webServerLoop() {
    ws.cleanupClients();
}
