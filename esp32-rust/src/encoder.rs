//! Rotary encoder input — polling-based with debounce.

use esp_idf_svc::hal::gpio::{AnyIOPin, Input, PinDriver, Pull};
use log::info;

/// Encoder events consumed by the main loop
#[derive(Debug, Clone, Copy)]
pub enum EncoderEvent {
    Rotate(i32),   // +1 = clockwise, -1 = counter-clockwise
    Press,
}

pub struct Encoder {
    clk: PinDriver<'static, AnyIOPin, Input>,
    dt: PinDriver<'static, AnyIOPin, Input>,
    sw: PinDriver<'static, AnyIOPin, Input>,
    last_clk: bool,
    last_button: bool,
    debounce_ms: u32,
    last_event_ms: u32,
    button_down_since: Option<u32>,
}

impl Encoder {
    pub fn new(
        clk_pin: AnyIOPin,
        dt_pin: AnyIOPin,
        sw_pin: AnyIOPin,
    ) -> Result<Self, esp_idf_svc::sys::EspError> {
        let mut clk = PinDriver::input(clk_pin)?;
        clk.set_pull(Pull::Up)?;

        let mut dt = PinDriver::input(dt_pin)?;
        dt.set_pull(Pull::Up)?;

        let mut sw = PinDriver::input(sw_pin)?;
        sw.set_pull(Pull::Up)?;

        let last_clk = clk.is_high();

        info!("Encoder: Initialized");

        Ok(Self {
            clk,
            dt,
            sw,
            last_clk,
            last_button: true,
            debounce_ms: 50,
            last_event_ms: 0,
            button_down_since: None,
        })
    }

    /// Poll for encoder events. Call from the main loop.
    pub fn poll(&mut self, now_ms: u32) -> Option<EncoderEvent> {
        if now_ms.wrapping_sub(self.last_event_ms) < self.debounce_ms {
            return None;
        }

        // Check rotation (gray code)
        let clk = self.clk.is_high();
        if clk != self.last_clk {
            self.last_clk = clk;
            if !clk {
                // Falling edge on CLK — check DT for direction
                let dt = self.dt.is_high();
                self.last_event_ms = now_ms;
                return Some(EncoderEvent::Rotate(if dt { 1 } else { -1 }));
            }
        }

        // Check button (active low)
        let button = self.sw.is_high();
        if !button && self.last_button {
            // Falling edge = press started
            self.button_down_since = Some(now_ms);
        } else if button && !self.last_button {
            // Rising edge = released
            if let Some(down_since) = self.button_down_since {
                self.button_down_since = None;
                if now_ms.wrapping_sub(down_since) < 10_000 {
                    // Short press
                    self.last_button = button;
                    self.last_event_ms = now_ms;
                    return Some(EncoderEvent::Press);
                }
                // Long press handled by is_long_hold()
            }
        }
        self.last_button = button;

        None
    }

    /// Check if the button is being held for 10+ seconds (factory reset)
    pub fn is_long_hold(&self, now_ms: u32) -> bool {
        if let Some(down_since) = self.button_down_since {
            now_ms.wrapping_sub(down_since) >= 10_000
        } else {
            false
        }
    }
}
