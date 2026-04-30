#ifndef OPENTHERM_FFI_H
#define OPENTHERM_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Response status codes
#define OT_STATUS_NONE     0
#define OT_STATUS_SUCCESS  1
#define OT_STATUS_INVALID  2
#define OT_STATUS_TIMEOUT  3

// Message types
#define OT_MSG_READ_DATA   0
#define OT_MSG_WRITE_DATA  1
#define OT_MSG_INVALID     2
#define OT_MSG_RESERVED    3
#define OT_MSG_READ_ACK    4
#define OT_MSG_WRITE_ACK   5
#define OT_MSG_DATA_INVALID 6
#define OT_MSG_UNKNOWN_ID  7

/// Initialize OpenTherm master with given GPIO pins.
/// Returns 0 on success, -1 on failure.
int ot_init(int in_pin, int out_pin);

/// Send a synchronous request and return the response.
/// Blocks until response received or timeout (~1s).
uint32_t ot_send_request(uint32_t request);

/// Get the status of the last response.
/// Returns one of OT_STATUS_* constants.
int ot_get_last_status(void);

/// Build an OpenTherm frame from message type, data ID, and data value.
uint32_t ot_build_request(int msg_type, int data_id, uint16_t data);

/// Extract message type from a frame.
int ot_get_message_type(uint32_t frame);

/// Extract data ID from a frame.
int ot_get_data_id(uint32_t frame);

/// Extract 16-bit data value from a frame.
uint16_t ot_get_data_value(uint32_t frame);

/// Clean up OpenTherm instance.
void ot_end(void);

#ifdef __cplusplus
}
#endif

#endif // OPENTHERM_FFI_H
