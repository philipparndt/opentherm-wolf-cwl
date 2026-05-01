/**
 * Arduino compatibility shim for ESP-IDF.
 * Minimal subset needed by the OpenTherm library.
 * Avoids FreeRTOS includes to keep compilation simple.
 */
#ifndef ARDUINO_H_COMPAT
#define ARDUINO_H_COMPAT

#include <stdint.h>
#include <stdbool.h>
#include <functional>

#include "esp_timer.h"
#include "esp_rom_sys.h"
#include "driver/gpio.h"

// Types
typedef uint8_t byte;

// Constants
#define HIGH 1
#define LOW  0
#define INPUT  0x01
#define OUTPUT 0x03
#define CHANGE GPIO_INTR_ANYEDGE

// Binary literals
#define B000 0
#define B001 1
#define B010 2
#define B011 3
#define B100 4
#define B101 5
#define B110 6
#define B111 7

// Bit manipulation
#define bitRead(value, bit) (((value) >> (bit)) & 0x01)

// IRAM_ATTR
#ifndef IRAM_ATTR
#define IRAM_ATTR __attribute__((section(".iram1.text")))
#endif

// --- GPIO ---

inline void pinMode(int pin, int mode) {
    gpio_config_t cfg = {};
    cfg.pin_bit_mask = (1ULL << pin);
    cfg.mode = (mode == OUTPUT) ? GPIO_MODE_OUTPUT : GPIO_MODE_INPUT;
    cfg.pull_up_en = GPIO_PULLUP_DISABLE;
    cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
    cfg.intr_type = GPIO_INTR_DISABLE;
    gpio_config(&cfg);
}

inline int digitalRead(int pin) {
    return gpio_get_level((gpio_num_t)pin);
}

inline void digitalWrite(int pin, int value) {
    gpio_set_level((gpio_num_t)pin, value);
}

// --- Interrupts ---

static void (*_ot_isr_cb)(void) = nullptr;
static std::function<void()> _ot_isr_fn;
static bool _ot_isr_installed = false;

static void IRAM_ATTR _ot_gpio_isr(void* arg) {
    if (_ot_isr_cb) _ot_isr_cb();
    else if (_ot_isr_fn) _ot_isr_fn();
}

inline int digitalPinToInterrupt(int pin) { return pin; }

inline void attachInterrupt(int pin, void (*cb)(void), int mode) {
    _ot_isr_cb = cb;
    _ot_isr_fn = nullptr;
    gpio_set_intr_type((gpio_num_t)pin, (gpio_int_type_t)mode);
    if (!_ot_isr_installed) { gpio_install_isr_service(0); _ot_isr_installed = true; }
    gpio_isr_handler_add((gpio_num_t)pin, _ot_gpio_isr, nullptr);
    gpio_intr_enable((gpio_num_t)pin);
}

inline void attachInterrupt(int pin, std::function<void()> cb, int mode) {
    _ot_isr_cb = nullptr;
    _ot_isr_fn = cb;
    gpio_set_intr_type((gpio_num_t)pin, (gpio_int_type_t)mode);
    if (!_ot_isr_installed) { gpio_install_isr_service(0); _ot_isr_installed = true; }
    gpio_isr_handler_add((gpio_num_t)pin, _ot_gpio_isr, nullptr);
    gpio_intr_enable((gpio_num_t)pin);
}

inline void detachInterrupt(int pin) {
    gpio_isr_handler_remove((gpio_num_t)pin);
    gpio_intr_disable((gpio_num_t)pin);
}

// --- Timing ---

inline unsigned long millis() {
    return (unsigned long)(esp_timer_get_time() / 1000);
}

inline unsigned long micros() {
    return (unsigned long)esp_timer_get_time();
}

inline void delay(unsigned long ms) {
    esp_rom_delay_us(ms * 1000);
}

inline void delayMicroseconds(unsigned long us) {
    esp_rom_delay_us(us);
}

// --- Interrupt control ---

inline void noInterrupts() {
    // On ESP32, use portDISABLE_INTERRUPTS via direct register access
    // to avoid FreeRTOS dependency
    __asm__ __volatile__("rsil a2, 3" ::: "a2");
}

inline void interrupts() {
    __asm__ __volatile__("rsil a2, 0" ::: "a2");
}

// --- Misc ---

inline void yield() {
    // Busy-wait noop — the sendRequest loop calls this
}

#endif // ARDUINO_H_COMPAT
