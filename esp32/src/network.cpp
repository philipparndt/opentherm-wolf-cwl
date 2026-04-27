#include "network.h"
#include "config_manager.h"
#include "display.h"
#include "logging.h"

// WiFi.h must be included before ETH.h (ETH.h depends on WiFi types)
#include <WiFi.h>

#if defined(USE_ETHERNET)
  #include <ETH.h>
#endif

bool networkConnected = false;

// =============================================================================
// Ethernet Implementation (Olimex ESP32-POE)
// =============================================================================

#if defined(USE_ETHERNET)

void onEthEvent(WiFiEvent_t event) {
  switch (event) {
    case ARDUINO_EVENT_ETH_START:
      log("ETH: Started");
      ETH.setHostname("wolf-cwl");
      break;
    case ARDUINO_EVENT_ETH_CONNECTED:
      log("ETH: Link up (" + String(ETH.linkSpeed()) + "Mbps " +
          (ETH.fullDuplex() ? "Full" : "Half") + " Duplex)");
      break;
    case ARDUINO_EVENT_ETH_GOT_IP:
      log("ETH: Got IP: " + ETH.localIP().toString() +
          " GW: " + ETH.gatewayIP().toString() +
          " DNS: " + ETH.dnsIP().toString());
      networkConnected = true;
      displayShowIP(ETH.localIP().toString().c_str());
      break;
    case ARDUINO_EVENT_ETH_DISCONNECTED:
      log("ETH: Link down");
      networkConnected = false;
      displayShowDisconnected();
      break;
    case ARDUINO_EVENT_ETH_STOP:
      log("ETH: Stopped");
      networkConnected = false;
      break;
    default:
      // Log unknown events for debugging
      Serial.println("ETH: Event " + String((int)event));
      break;
  }
}

void setupNetwork() {
  log("Initializing Ethernet...");
  // Disable WiFi STA so WiFiClient routes through ETH, not a dead WiFi interface
  WiFi.mode(WIFI_OFF);
  WiFi.onEvent(onEthEvent);
  ETH.begin();
}

static unsigned long lastEthCheck = 0;
#define ETH_CHECK_INTERVAL 10000

void networkLoop() {
  // Periodic Ethernet state check (in case events are missed)
  unsigned long now = millis();
  if (now - lastEthCheck > ETH_CHECK_INTERVAL) {
    lastEthCheck = now;
    bool linked = ETH.linkUp();
    if (linked && !networkConnected) {
      log("ETH: Link recovered (polling fallback)");
      // ETH.begin() is already done, DHCP should re-acquire
    } else if (!linked && networkConnected) {
      log("ETH: Link lost (polling fallback)");
      networkConnected = false;
    }
  }
}

// =============================================================================
// WiFi Implementation
// =============================================================================

#elif defined(USE_WIFI)

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
      displayShowIP(WiFi.localIP().toString().c_str());
    }
  } else {
    if (networkConnected) {
      log("WiFi: Disconnected");
      networkConnected = false;
      displayShowDisconnected();
    }

    if (now - lastWifiCheck > WIFI_CHECK_INTERVAL) {
      lastWifiCheck = now;
      log("WiFi: Attempting reconnect...");
      WiFi.disconnect();
      WiFi.begin(appConfig.wifiSsid, appConfig.wifiPassword);
    }
  }
}

#else
  #error "Please define USE_ETHERNET or USE_WIFI in build flags"
#endif
