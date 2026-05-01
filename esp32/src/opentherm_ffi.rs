//! FFI bindings to the C++ OpenTherm library.
//!
//! Provides a safe Rust wrapper around the C-linkage functions in opentherm_ffi.cpp.

#[allow(dead_code)]
mod ffi {
    extern "C" {
        pub fn ot_init(in_pin: i32, out_pin: i32) -> i32;
        pub fn ot_send_request(request: u32) -> u32;
        pub fn ot_get_last_status() -> i32;
        pub fn ot_build_request(msg_type: i32, data_id: i32, data: u16) -> u32;
        pub fn ot_get_message_type(frame: u32) -> i32;
        pub fn ot_get_data_id(frame: u32) -> i32;
        pub fn ot_get_data_value(frame: u32) -> u16;
        pub fn ot_end();
    }
}

/// OpenTherm response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    None,
    Success,
    Invalid,
    Timeout,
}

/// OpenTherm message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    ReadData = 0,
    WriteData = 1,
    InvalidData = 2,
    Reserved = 3,
    ReadAck = 4,
    WriteAck = 5,
    DataInvalid = 6,
    UnknownDataId = 7,
}

/// Safe wrapper around the OpenTherm C++ library
pub struct OpenTherm {
    _initialized: bool,
}

impl OpenTherm {
    /// Initialize OpenTherm master with the given GPIO pins.
    pub fn new(in_pin: i32, out_pin: i32) -> Result<Self, &'static str> {
        let result = unsafe { ffi::ot_init(in_pin, out_pin) };
        if result == 0 {
            Ok(Self { _initialized: true })
        } else {
            Err("Failed to initialize OpenTherm")
        }
    }

    /// Send a synchronous request and return the response.
    /// Blocks until response received or timeout (~1s).
    pub fn send_request(&self, request: u32) -> u32 {
        unsafe { ffi::ot_send_request(request) }
    }

    /// Get the status of the last response.
    pub fn last_status(&self) -> ResponseStatus {
        match unsafe { ffi::ot_get_last_status() } {
            1 => ResponseStatus::Success,
            2 => ResponseStatus::Invalid,
            3 => ResponseStatus::Timeout,
            _ => ResponseStatus::None,
        }
    }

    /// Build an OpenTherm frame from components.
    pub fn build_request(msg_type: MessageType, data_id: u8, data: u16) -> u32 {
        unsafe { ffi::ot_build_request(msg_type as i32, data_id as i32, data) }
    }

    /// Extract message type from a frame.
    pub fn get_message_type(frame: u32) -> MessageType {
        match unsafe { ffi::ot_get_message_type(frame) } {
            0 => MessageType::ReadData,
            1 => MessageType::WriteData,
            2 => MessageType::InvalidData,
            3 => MessageType::Reserved,
            4 => MessageType::ReadAck,
            5 => MessageType::WriteAck,
            6 => MessageType::DataInvalid,
            7 => MessageType::UnknownDataId,
            _ => MessageType::InvalidData,
        }
    }

    /// Extract data ID from a frame.
    pub fn get_data_id(frame: u32) -> u8 {
        unsafe { ffi::ot_get_data_id(frame) as u8 }
    }

    /// Extract 16-bit data value from a frame.
    pub fn get_data_value(frame: u32) -> u16 {
        unsafe { ffi::ot_get_data_value(frame) }
    }
}

impl Drop for OpenTherm {
    fn drop(&mut self) {
        unsafe { ffi::ot_end() }
    }
}
