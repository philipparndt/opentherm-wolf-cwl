#include "network.h"
#include "config_manager.h"
#include "logging.h"
#include <WiFi.h>

bool networkConnected = false;

static unsigned long lastWifiCheck = 0;
#define WIFI_CHECK_INTERVAL 10000

void setupNetwork() {
  log("Initializing WiFi...");

  if (!hasWifiCredentials()) {
    log("WiFi: No credentials configured");
    return;
  }

  WiFi.mode(WIFI_STA);
  WiFi.setHostname("wolf-cwl");
  WiFi.begin(appConfig.wifiSsid, appConfig.wifiPassword);

  Serial.print("Connecting to WiFi: " + String(appConfig.wifiSsid));
  int attempts = 0;
  while (WiFi.status() != WL_CONNECTED && attempts < 30) {
    delay(500);
    Serial.print(".");
    attempts++;
  }
  Serial.println();

  if (WiFi.status() == WL_CONNECTED) {
    log("WiFi: Connected, IP: " + WiFi.localIP().toString());
    WiFi.setSleep(false);
    networkConnected = true;
  } else {
    log("WiFi: Connection failed, will retry...");
  }
}

void networkLoop() {
  unsigned long now = millis();

  if (!hasWifiCredentials()) {
    return;
  }

  if (WiFi.status() == WL_CONNECTED) {
    if (!networkConnected) {
      log("WiFi: Reconnected, IP: " + WiFi.localIP().toString());
      WiFi.setSleep(false);
      networkConnected = true;
    }
  } else {
    if (networkConnected) {
      log("WiFi: Disconnected");
      networkConnected = false;
    }

    if (now - lastWifiCheck > WIFI_CHECK_INTERVAL) {
      lastWifiCheck = now;
      log("WiFi: Attempting reconnect...");
      WiFi.disconnect();
      WiFi.begin(appConfig.wifiSsid, appConfig.wifiPassword);
    }
  }
}
