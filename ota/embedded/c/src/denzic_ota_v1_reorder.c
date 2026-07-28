#include "denzic_ota_v1_reorder.h"

#include <string.h>

static uint32_t denzic_ota_v1_read_u32_le(const uint8_t *bytes)
{
    return (uint32_t)bytes[0]
        | ((uint32_t)bytes[1] << 8)
        | ((uint32_t)bytes[2] << 16)
        | ((uint32_t)bytes[3] << 24);
}

void denzic_ota_v1_reorder_init(
    denzic_ota_v1_reorder_t *reorder,
    denzic_ota_v1_reorder_slot_t *slots,
    uint16_t slot_count,
    uint16_t max_payload_bytes)
{
    if (reorder == NULL) {
        return;
    }
    reorder->slots = slots;
    reorder->slot_count = slots == NULL ? 0u : slot_count;
    reorder->used = 0u;
    reorder->max_payload_bytes =
        max_payload_bytes == 0u ? 500u : max_payload_bytes;
    denzic_ota_v1_reorder_clear(reorder);
}

void denzic_ota_v1_reorder_clear(denzic_ota_v1_reorder_t *reorder)
{
    if (reorder == NULL || reorder->slots == NULL) {
        return;
    }
    for (uint16_t i = 0u; i < reorder->slot_count; i++) {
        reorder->slots[i].valid = false;
        reorder->slots[i].length = 0u;
    }
    reorder->used = 0u;
}

static int denzic_ota_v1_reorder_find_offset(
    const denzic_ota_v1_reorder_t *reorder,
    uint32_t offset)
{
    if (reorder == NULL || reorder->slots == NULL) {
        return -1;
    }
    for (uint16_t i = 0u; i < reorder->slot_count; i++) {
        if (!reorder->slots[i].valid) {
            continue;
        }
        if (reorder->slots[i].length <= DENZIC_OTA_V1_DATA_HEADER_BYTES) {
            continue;
        }
        if (denzic_ota_v1_read_u32_le(reorder->slots[i].data) == offset) {
            return (int)i;
        }
    }
    return -1;
}

static int denzic_ota_v1_reorder_find_free(
    const denzic_ota_v1_reorder_t *reorder)
{
    if (reorder == NULL || reorder->slots == NULL) {
        return -1;
    }
    for (uint16_t i = 0u; i < reorder->slot_count; i++) {
        if (!reorder->slots[i].valid) {
            return (int)i;
        }
    }
    return -1;
}

void denzic_ota_v1_reorder_drain(
    denzic_ota_v1_context_t *context,
    denzic_ota_v1_reorder_t *reorder)
{
    if (context == NULL || reorder == NULL || reorder->slots == NULL) {
        return;
    }
    while (reorder->used > 0u) {
        int slot = denzic_ota_v1_reorder_find_offset(reorder, context->bytes_written);
        if (slot < 0) {
            break;
        }
        (void)denzic_ota_v1_handle_data(
            context,
            reorder->slots[slot].data,
            reorder->slots[slot].length);
        reorder->slots[slot].valid = false;
        reorder->slots[slot].length = 0u;
        if (reorder->used > 0u) {
            reorder->used--;
        }
    }
}

bool denzic_ota_v1_handle_data_ordered(
    denzic_ota_v1_context_t *context,
    denzic_ota_v1_reorder_t *reorder,
    const uint8_t *bytes,
    size_t length)
{
    if (context == NULL) {
        return false;
    }
    if (reorder == NULL || reorder->slots == NULL || reorder->slot_count == 0u) {
        return denzic_ota_v1_handle_data(context, bytes, length);
    }
    if (bytes == NULL || length <= DENZIC_OTA_V1_DATA_HEADER_BYTES) {
        return denzic_ota_v1_handle_data(context, bytes, length);
    }
    if (context->state != DENZIC_OTA_V1_STATE_RECEIVING) {
        return denzic_ota_v1_handle_data(context, bytes, length);
    }

    const uint32_t offset = denzic_ota_v1_read_u32_le(bytes);
    const uint32_t expected = context->bytes_written;

    if (offset < expected) {
        bool ok = denzic_ota_v1_handle_data(context, bytes, length);
        denzic_ota_v1_reorder_drain(context, reorder);
        return ok;
    }
    if (offset == expected) {
        bool ok = denzic_ota_v1_handle_data(context, bytes, length);
        denzic_ota_v1_reorder_drain(context, reorder);
        return ok;
    }

    const uint32_t max_ahead =
        (uint32_t)reorder->slot_count * (uint32_t)reorder->max_payload_bytes;
    if (offset - expected > max_ahead) {
        return denzic_ota_v1_handle_data(context, bytes, length);
    }
    if (denzic_ota_v1_reorder_find_offset(reorder, offset) >= 0) {
        return true;
    }

    int free_slot = denzic_ota_v1_reorder_find_free(reorder);
    if (free_slot < 0) {
        return denzic_ota_v1_handle_data(context, bytes, length);
    }
    if (length > DENZIC_OTA_V1_REORDER_MAX_PACKET_BYTES) {
        return denzic_ota_v1_handle_data(context, bytes, length);
    }

    memcpy(reorder->slots[free_slot].data, bytes, length);
    reorder->slots[free_slot].length = (uint16_t)length;
    reorder->slots[free_slot].valid = true;
    if (reorder->used < reorder->slot_count) {
        reorder->used++;
    }
    return true;
}
