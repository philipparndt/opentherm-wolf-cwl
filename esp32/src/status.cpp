#include "status.h"
#include "logging.h"
#include "config.h"
#include "network.h"
#include "mqtt_client.h"
#include <esp_system.h>

#define LED_BLINK_INTERVAL 250

static unsigned long lastLedToggle = 0;
static bool ledState = false;
static int currentMode = -1;  // -1=unknown, 0=off, 1=connected

void setupStatusLed() {
  #if PIN_STATUS_LED >= 0
    pinMode(PIN_STATUS_LED, OUTPUT);
    digitalWrite(PIN_STATUS_LED, LOW);
    log("Status LED: GPIO " + String(PIN_STATUS_LED));
  #else
    log("Status LED: disabled");
  #endif
}

void printSystemStatus() {
  uint32_t freeHeap = ESP.getFreeHeap();
  uint32_t minFreeHeap = ESP.getMinFreeHeap();
  uint32_t heapSize = ESP.getHeapSize();
  uint32_t usedHeap = heapSize - freeHeap;
  float heapUsagePercent = (float)usedHeap / heapSize * 100.0;

  logDebug("--- System Status ---");
  logDebug("  Heap: " + String(usedHeap / 1024) + "KB / " + String(heapSize / 1024) + "KB (" + String(heapUsagePercent, 1) + "% used)");
  logDebug("  Free: " + String(freeHeap / 1024) + "KB, Min free: " + String(minFreeHeap / 1024) + "KB");
  logDebug("  CPU: " + String(ESP.getCpuFreqMHz()) + " MHz");
  logDebug("  Uptime: " + String(millis() / 1000 / 60) + " min");
  logDebug("  MQTT: " + String(mqtt.connected() ? "connected" : "disconnected"));
}

void updateStatusLed() {
  #if PIN_STATUS_LED >= 0
    bool shouldBeOn = networkConnected && mqtt.connected();
    if (ledState != shouldBeOn) {
      ledState = shouldBeOn;
      digitalWrite(PIN_STATUS_LED, ledState ? HIGH : LOW);
    }
  #endif
}
