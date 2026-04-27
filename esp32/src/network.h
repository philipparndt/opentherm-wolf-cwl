#pragma once

#include <Arduino.h>
#include <WiFi.h>
#include "config.h"

// Network state
extern bool networkConnected;

// Network functions
void setupNetwork();
void networkLoop();
