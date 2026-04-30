//! Status LED — on when network + MQTT connected.

use esp_idf_svc::hal::gpio::{AnyIOPin, Output, PinDriver};

pub struct StatusLed {
    pin: Option<PinDriver<'static, AnyIOPin, Output>>,
    current: bool,
}

impl StatusLed {
    pub fn new(pin: AnyIOPin) -> Self {
        match PinDriver::output(pin) {
            Ok(mut p) => {
                p.set_low().ok();
                Self { pin: Some(p), current: false }
            }
            Err(_) => Self { pin: None, current: false },
        }
    }

    pub fn set(&mut self, on: bool) {
        if on == self.current { return; }
        self.current = on;
        if let Some(ref mut p) = self.pin {
            if on { p.set_high().ok(); } else { p.set_low().ok(); }
        }
    }
}
