#include "encoder.h"
#include "config_manager.h"
#include "display.h"
#include "logging.h"

// Encoder state
static volatile int encoderPosition = 0;
static volatile bool encoderChanged = false;
static int lastPosition = 0;

// Button state
static bool lastButtonState = HIGH;
static unsigned long lastDebounceTime = 0;
static unsigned long buttonPressStart = 0;
static bool buttonHandled = false;
#define DEBOUNCE_MS 50

// Pin state for ISR
static uint8_t encClkPin;
static uint8_t encDtPin;

// =============================================================================
// Interrupt handler for rotary encoder
// =============================================================================
static void IRAM_ATTR encoderISR() {
    static uint8_t lastClk = HIGH;
    uint8_t clk = digitalRead(encClkPin);

    if (clk != lastClk && clk == LOW) {
        if (digitalRead(encDtPin) != clk) {
            encoderPosition++;
        } else {
            encoderPosition--;
        }
        encoderChanged = true;
    }
    lastClk = clk;
}

void setupEncoder() {
    encClkPin = appConfig.encClkPin;
    encDtPin = appConfig.encDtPin;

    pinMode(encClkPin, INPUT_PULLUP);
    pinMode(encDtPin, INPUT_PULLUP);
    pinMode(appConfig.encSwPin, INPUT_PULLUP);

    attachInterrupt(digitalPinToInterrupt(encClkPin), encoderISR, CHANGE);

    log("Encoder: Initialized (CLK=" + String(encClkPin) +
        ", DT=" + String(encDtPin) +
        ", SW=" + String(appConfig.encSwPin) + ")");
}

void encoderLoop() {
    // Handle rotation
    if (encoderChanged) {
        encoderChanged = false;
        int pos = encoderPosition;
        int delta = pos - lastPosition;
        lastPosition = pos;

        if (delta != 0) {
            if (editMode) {
                adjustEditValue(delta > 0 ? 1 : -1);
            } else {
                if (delta > 0) nextPage();
                else prevPage();
            }
        }
    }

    // Handle button press (with debounce)
    bool buttonState = digitalRead(appConfig.encSwPin);

    if (buttonState != lastButtonState) {
        lastDebounceTime = millis();
    }

    if (millis() - lastDebounceTime > DEBOUNCE_MS) {
        if (buttonState == LOW && !buttonHandled) {
            // Button pressed
            buttonPressStart = millis();
            buttonHandled = true;

            if (editMode) {
                exitEditMode(true);  // Confirm
            } else {
                enterEditMode();
            }
        }

        if (buttonState == HIGH) {
            buttonHandled = false;
        }
    }

    lastButtonState = buttonState;
}
