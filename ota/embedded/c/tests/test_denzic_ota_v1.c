#include "denzic_ota_v1.h"

#include <assert.h>
#include <string.h>

typedef struct {
    uint8_t image[64];
    size_t image_size;
    uint32_t expected_size;
    bool aborted;
    bool finished;
} test_storage_t;

static bool storage_begin(void *driver_context, uint32_t image_size)
{
    test_storage_t *storage = (test_storage_t *)driver_context;
    storage->image_size = 0u;
    storage->expected_size = image_size;
    storage->aborted = false;
    storage->finished = false;
    return image_size <= sizeof(storage->image);
}

static bool storage_write(void *driver_context, const uint8_t *data, size_t length)
{
    test_storage_t *storage = (test_storage_t *)driver_context;
    if (storage->image_size + length > sizeof(storage->image)) {
        return false;
    }
    memcpy(&storage->image[storage->image_size], data, length);
    storage->image_size += length;
    return true;
}

static bool storage_finish(void *driver_context)
{
    test_storage_t *storage = (test_storage_t *)driver_context;
    storage->finished = storage->image_size == storage->expected_size;
    return storage->finished;
}

static void storage_abort(void *driver_context)
{
    ((test_storage_t *)driver_context)->aborted = true;
}

static void write_u16(uint8_t *bytes, uint16_t value)
{
    bytes[0] = (uint8_t)(value & 0xffu);
    bytes[1] = (uint8_t)(value >> 8u);
}

static void write_u32(uint8_t *bytes, uint32_t value)
{
    bytes[0] = (uint8_t)(value & 0xffu);
    bytes[1] = (uint8_t)(value >> 8u);
    bytes[2] = (uint8_t)(value >> 16u);
    bytes[3] = (uint8_t)(value >> 24u);
}

static void control_packet(uint8_t *packet, uint8_t operation, uint32_t size, uint16_t chunk, uint16_t window)
{
    memset(packet, 0, DENZIC_OTA_V1_CONTROL_BYTES);
    memcpy(packet, DENZIC_OTA_V1_MAGIC, 4u);
    packet[4] = operation;
    packet[5] = DENZIC_OTA_V1_PROTOCOL_VERSION;
    write_u32(&packet[8], size);
    write_u16(&packet[12], chunk);
    write_u16(&packet[14], window);
}

int main(void)
{
    denzic_ota_v1_context_t context;
    test_storage_t storage = {0};
    denzic_ota_v1_storage_driver_t driver = {
        storage_begin,
        storage_write,
        storage_finish,
        storage_abort,
    };
    uint8_t control[DENZIC_OTA_V1_CONTROL_BYTES];
    uint8_t data[DENZIC_OTA_V1_DATA_HEADER_BYTES + 5u];
    uint8_t status[DENZIC_OTA_V1_STATUS_BYTES];

    denzic_ota_v1_init(&context, driver, &storage, 5u, 3u);
    control_packet(control, DENZIC_OTA_V1_OP_BEGIN, 8u, 5u, 3u);
    assert(denzic_ota_v1_handle_control(&context, control, sizeof(control)));
    assert(context.state == DENZIC_OTA_V1_STATE_RECEIVING);

    write_u32(data, 0u);
    memcpy(&data[4], "abcde", 5u);
    assert(denzic_ota_v1_handle_data(&context, data, sizeof(data)));
    assert(context.bytes_written == 5u);

    write_u32(data, 7u);
    assert(!denzic_ota_v1_handle_data(&context, data, sizeof(data)));
    assert(context.state == DENZIC_OTA_V1_STATE_RECEIVING);
    assert(context.last_error == DENZIC_OTA_V1_ERROR_OFFSET_MISMATCH);

    write_u32(data, 5u);
    memcpy(&data[4], "fgh", 3u);
    assert(denzic_ota_v1_handle_data(&context, data, 7u));
    assert(context.bytes_written == 8u);
    assert(context.last_error == DENZIC_OTA_V1_ERROR_NONE);

    control_packet(control, DENZIC_OTA_V1_OP_FINISH, 8u, 5u, 3u);
    assert(denzic_ota_v1_handle_control(&context, control, sizeof(control)));
    assert(storage.finished);
    assert(context.state == DENZIC_OTA_V1_STATE_COMPLETE);
    denzic_ota_v1_set_status_flags(&context, DENZIC_OTA_V1_STATUS_FLAG_ACTIVE_LINK_CONFIRMED);
    assert(denzic_ota_v1_encode_status(&context, status, sizeof(status)) == sizeof(status));
    assert(memcmp(status, "DOV1", 4u) == 0);
    assert(status[4] == 1u);
    assert(status[5] == DENZIC_OTA_V1_STATE_COMPLETE);
    assert(status[7] == DENZIC_OTA_V1_STATUS_FLAG_ACTIVE_LINK_CONFIRMED);

    denzic_ota_v1_reset(&context);
    memcpy(control, "COV2", 4u);
    assert(!denzic_ota_v1_handle_control(&context, control, sizeof(control)));
    assert(context.last_error == DENZIC_OTA_V1_ERROR_BAD_MAGIC);
    return 0;
}
