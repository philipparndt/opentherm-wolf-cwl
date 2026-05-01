#include "opentherm_ffi.h"
#include "OpenTherm.h"

static OpenTherm* ot_instance = nullptr;

extern "C" {

int ot_init(int in_pin, int out_pin) {
    if (ot_instance) {
        ot_instance->end();
        delete ot_instance;
    }
    ot_instance = new OpenTherm(in_pin, out_pin, false);
    ot_instance->begin();
    return 0;
}

uint32_t ot_send_request(uint32_t request) {
    if (!ot_instance) return 0;
    return (uint32_t)ot_instance->sendRequest((unsigned long)request);
}

int ot_get_last_status(void) {
    if (!ot_instance) return OT_STATUS_NONE;
    switch (ot_instance->getLastResponseStatus()) {
        case OpenThermResponseStatus::NONE:    return OT_STATUS_NONE;
        case OpenThermResponseStatus::SUCCESS: return OT_STATUS_SUCCESS;
        case OpenThermResponseStatus::INVALID: return OT_STATUS_INVALID;
        case OpenThermResponseStatus::TIMEOUT: return OT_STATUS_TIMEOUT;
        default: return OT_STATUS_NONE;
    }
}

uint32_t ot_build_request(int msg_type, int data_id, uint16_t data) {
    return (uint32_t)OpenTherm::buildRequest(
        (OpenThermMessageType)msg_type,
        (OpenThermMessageID)data_id,
        (unsigned int)data
    );
}

int ot_get_message_type(uint32_t frame) {
    return (int)OpenTherm::getMessageType((unsigned long)frame);
}

int ot_get_data_id(uint32_t frame) {
    return (int)OpenTherm::getDataID((unsigned long)frame);
}

uint16_t ot_get_data_value(uint32_t frame) {
    return (uint16_t)(frame & 0xFFFF);
}

void ot_end(void) {
    if (ot_instance) {
        ot_instance->end();
        delete ot_instance;
        ot_instance = nullptr;
    }
}

} // extern "C"
