#include "denzic_ota_v1.h"
#include "denzic_ota_v1_reorder.h"

#include <assert.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    uint8_t storage[64];
    size_t used;
} sink_t;

static bool sink_begin(void *ctx, uint32_t image_size)
{
    sink_t *sink = (sink_t *)ctx;
    (void)image_size;
    sink->used = 0u;
    return true;
}

static bool sink_write(void *ctx, const uint8_t *data, size_t length)
{
    sink_t *sink = (sink_t *)ctx;
    if (sink->used + length > sizeof(sink->storage)) {
        return false;
    }
    memcpy(&sink->storage[sink->used], data, length);
    sink->used += length;
    return true;
}

static bool sink_finish(void *ctx)
{
    (void)ctx;
    return true;
}

static void sink_abort(void *ctx)
{
    sink_t *sink = (sink_t *)ctx;
    sink->used = 0u;
}

static void write_u32_le(uint8_t *out, uint32_t value)
{
    out[0] = (uint8_t)(value & 0xffu);
    out[1] = (uint8_t)((value >> 8) & 0xffu);
    out[2] = (uint8_t)((value >> 16) & 0xffu);
    out[3] = (uint8_t)((value >> 24) & 0xffu);
}

static void make_control_begin(uint8_t *control, uint32_t size)
{
    memset(control, 0, DENZIC_OTA_V1_CONTROL_BYTES);
    memcpy(control, DENZIC_OTA_V1_MAGIC, 4u);
    control[4] = DENZIC_OTA_V1_OP_BEGIN;
    control[5] = DENZIC_OTA_V1_PROTOCOL_VERSION;
    write_u32_le(&control[8], size);
    control[12] = 4u;
    control[13] = 0u;
    control[14] = 8u;
    control[15] = 0u;
    write_u32_le(&control[16], 0x12345678u);
}

static void make_data(uint8_t *packet, uint32_t offset, const char *payload)
{
    write_u32_le(packet, offset);
    memcpy(&packet[DENZIC_OTA_V1_DATA_HEADER_BYTES], payload, 4u);
}

int main(void)
{
    sink_t sink = {0};
    denzic_ota_v1_storage_driver_t driver = {
        sink_begin,
        sink_write,
        sink_finish,
        sink_abort,
    };
    denzic_ota_v1_context_t context;
    denzic_ota_v1_reorder_slot_t slots[4];
    denzic_ota_v1_reorder_t reorder;
    uint8_t control[DENZIC_OTA_V1_CONTROL_BYTES];
    uint8_t data[DENZIC_OTA_V1_DATA_HEADER_BYTES + 4u];

    denzic_ota_v1_init(&context, driver, &sink, 4u, 8u);
    denzic_ota_v1_reorder_init(&reorder, slots, 4u, 4u);
    make_control_begin(control, 8u);
    assert(denzic_ota_v1_handle_control(&context, control, sizeof(control)));

    /* Out-of-order dual-lane: offset 4 arrives before offset 0. */
    make_data(data, 4u, "efgh");
    assert(denzic_ota_v1_handle_data_ordered(&context, &reorder, data, sizeof(data)));
    assert(context.bytes_written == 0u);
    assert(reorder.used == 1u);
    assert(context.last_error == DENZIC_OTA_V1_ERROR_NONE);

    make_data(data, 0u, "abcd");
    assert(denzic_ota_v1_handle_data_ordered(&context, &reorder, data, sizeof(data)));
    assert(context.bytes_written == 8u);
    assert(reorder.used == 0u);
    assert(memcmp(sink.storage, "abcdefgh", 8u) == 0);

    /* Direct core still rejects bare mismatch without reorder. */
    denzic_ota_v1_reset(&context);
    denzic_ota_v1_reorder_clear(&reorder);
    make_control_begin(control, 8u);
    assert(denzic_ota_v1_handle_control(&context, control, sizeof(control)));
    make_data(data, 4u, "efgh");
    assert(!denzic_ota_v1_handle_data(&context, data, sizeof(data)));
    assert(context.last_error == DENZIC_OTA_V1_ERROR_OFFSET_MISMATCH);

    return 0;
}
