#include "ap_mode.h"
#include "config_manager.h"
#include "logging.h"
#include <WiFi.h>
#include <DNSServer.h>

bool apModeActive = false;

static DNSServer dnsServer;
static const byte DNS_PORT = 53;

bool shouldStartApMode() {
  #if defined(USE_ETHERNET)
    return false;  // Ethernet boards don't need AP mode
  #else
    return !appConfig.configured && !hasWifiCredentials();
  #endif
}

void setupApMode() {
  // Generate AP name from MAC address
  uint8_t mac[6];
  WiFi.macAddress(mac);
  char apName[20];
  snprintf(apName, sizeof(apName), "WolfCWL-%02X%02X", mac[4], mac[5]);

  log("AP Mode: Starting AP '" + String(apName) + "'");

  WiFi.mode(WIFI_AP);
  WiFi.softAP(apName, "wolfcwl123");

  delay(100);  // Wait for AP to start

  // Start DNS server for captive portal
  dnsServer.start(DNS_PORT, "*", WiFi.softAPIP());

  log("AP Mode: IP " + WiFi.softAPIP().toString());
  apModeActive = true;
}

void stopApMode() {
  dnsServer.stop();
  WiFi.softAPdisconnect(true);
  apModeActive = false;
  log("AP Mode: Stopped");
}

void apModeLoop() {
  dnsServer.processNextRequest();
}
