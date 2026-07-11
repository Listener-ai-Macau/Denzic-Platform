#include "denzic_ota_v1.h"

#include <string.h>

static uint16_t read_u16_le(const uint8_t *bytes)
{
    return (uint16_t)((uint16_t)bytes[0] | ((uint16_t)bytes[1] << 8u));
}

static uint32_t read_u32_le(const uint8_t *bytes)
{
    return (uint32_t)bytes[0] |
           ((uint32_t)bytes[1] << 8u) |
           ((uint32_t)bytes[2] << 16u) |
           ((uint32_t)bytes[3] << 24u);
}

static void write_u16_le(uint8_t *bytes, uint16_t value)
{
    bytes[0] = (uint8_t)(value & 0xffu);
    bytes[1] = (uint8_t)((value >> 8u) & 0xffu);
}

static void write_u32_le(uint8_t *bytes, uint32_t value)
{
    bytes[0] = (uint8_t)(value & 0xffu);
    bytes[1] = (uint8_t)((value >> 8u) & 0xffu);
    bytes[2] = (uint8_t)((value >> 16u) & 0xffu);
    bytes[3] = (uint8_t)((value >> 24u) & 0xffu);
}

static bool has_magic(const uint8_t *bytes, size_t length)
{
    static const uint8_t magic[4] = DENZIC_OTA_V1_MAGIC;
    return bytes != NULL && length >= 4u && memcmp(bytes, magic, sizeof(magic)) == 0;
}

static void set_error(denzic_ota_v1_context_t *context, uint8_t error)
{
    context->state = DENZIC_OTA_V1_STATE_ERROR;
    context->last_error = error;
}

static void set_recoverable_error(denzic_ota_v1_context_t *context, uint8_t error)
{
    context->last_error = error;
}

void denzic_ota_v1_init(
    denzic_ota_v1_context_t *context,
    denzic_ota_v1_storage_driver_t driver,
    void *driver_context,
    uint16_t max_chunk_payload_bytes,
    uint16_t default_window_chunks)
{
    if (context == NULL) {
        return;
    }
    memset(context, 0, sizeof(*context));
    context->driver = driver;
    context->driver_context = driver_context;
    context->max_chunk_payload_bytes = max_chunk_payload_bytes;
    context->default_window_chunks = default_window_chunks;
    denzic_ota_v1_reset(context);
}

void denzic_ota_v1_reset(denzic_ota_v1_context_t *context)
{
    if (context == NULL) {
        return;
    }
    context->expected_size = 0u;
    context->bytes_written = 0u;
    context->data_write_count = 0u;
    context->chunk_payload_bytes = context->max_chunk_payload_bytes;
    context->window_chunks = context->default_window_chunks;
    context->state = DENZIC_OTA_V1_STATE_IDLE;
    context->last_error = DENZIC_OTA_V1_ERROR_NONE;
    context->status_flags = 0u;
}

void denzic_ota_v1_set_status_flags(
    denzic_ota_v1_context_t *context,
    uint8_t status_flags)
{
    if (context != NULL) {
        context->status_flags = status_flags;
    }
}

static bool handle_begin(
    denzic_ota_v1_context_t *context,
    const uint8_t *bytes,
    size_t length)
{
    uint32_t expected_size;
    uint16_t chunk_payload_bytes;
    uint16_t window_chunks;

    if (length < DENZIC_OTA_V1_CONTROL_BYTES) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_SIZE);
        return false;
    }
    expected_size = read_u32_le(&bytes[8]);
    chunk_payload_bytes = read_u16_le(&bytes[12]);
    window_chunks = read_u16_le(&bytes[14]);
    if (expected_size == 0u ||
        chunk_payload_bytes == 0u ||
        chunk_payload_bytes > context->max_chunk_payload_bytes) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_SIZE);
        return false;
    }
    if (window_chunks == 0u) {
        window_chunks = context->default_window_chunks;
    }
    if (window_chunks == 0u) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_SIZE);
        return false;
    }

    if (context->state == DENZIC_OTA_V1_STATE_RECEIVING && context->driver.abort != NULL) {
        context->driver.abort(context->driver_context);
    }
    denzic_ota_v1_reset(context);
    context->state = DENZIC_OTA_V1_STATE_ERASING;
    context->expected_size = expected_size;
    context->chunk_payload_bytes = chunk_payload_bytes;
    context->window_chunks = window_chunks;
    if (context->driver.begin == NULL ||
        !context->driver.begin(context->driver_context, expected_size)) {
        set_error(context, DENZIC_OTA_V1_ERROR_STORAGE_BEGIN);
        return false;
    }
    context->state = DENZIC_OTA_V1_STATE_RECEIVING;
    return true;
}

static bool handle_finish(denzic_ota_v1_context_t *context)
{
    if (context->state != DENZIC_OTA_V1_STATE_RECEIVING) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_STATE);
        return false;
    }
    if (context->bytes_written != context->expected_size) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_SIZE);
        return false;
    }
    if (context->driver.finish == NULL || !context->driver.finish(context->driver_context)) {
        set_error(context, DENZIC_OTA_V1_ERROR_STORAGE_FINISH);
        return false;
    }
    context->state = DENZIC_OTA_V1_STATE_COMPLETE;
    context->last_error = DENZIC_OTA_V1_ERROR_NONE;
    return true;
}

bool denzic_ota_v1_handle_control(
    denzic_ota_v1_context_t *context,
    const uint8_t *bytes,
    size_t length)
{
    if (context == NULL || bytes == NULL) {
        return false;
    }
    if (!has_magic(bytes, length) || length < 6u || bytes[5] != DENZIC_OTA_V1_PROTOCOL_VERSION) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_MAGIC);
        return false;
    }

    switch (bytes[4]) {
        case DENZIC_OTA_V1_OP_BEGIN:
            return handle_begin(context, bytes, length);
        case DENZIC_OTA_V1_OP_SYNC:
            if (context->state != DENZIC_OTA_V1_STATE_RECEIVING) {
                set_error(context, DENZIC_OTA_V1_ERROR_BAD_STATE);
                return false;
            }
            return true;
        case DENZIC_OTA_V1_OP_FINISH:
            return handle_finish(context);
        case DENZIC_OTA_V1_OP_ABORT:
            if (context->driver.abort != NULL) {
                context->driver.abort(context->driver_context);
            }
            denzic_ota_v1_reset(context);
            return true;
        default:
            set_error(context, DENZIC_OTA_V1_ERROR_BAD_MAGIC);
            return false;
    }
}

bool denzic_ota_v1_handle_data(
    denzic_ota_v1_context_t *context,
    const uint8_t *bytes,
    size_t length)
{
    uint32_t offset;
    uint32_t payload_length;

    if (context == NULL || bytes == NULL) {
        return false;
    }
    if (context->state != DENZIC_OTA_V1_STATE_RECEIVING) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_STATE);
        return false;
    }
    if (length <= DENZIC_OTA_V1_DATA_HEADER_BYTES ||
        length > (size_t)context->chunk_payload_bytes + DENZIC_OTA_V1_DATA_HEADER_BYTES) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_SIZE);
        return false;
    }

    offset = read_u32_le(bytes);
    payload_length = (uint32_t)(length - DENZIC_OTA_V1_DATA_HEADER_BYTES);
    if (offset < context->bytes_written) {
        if (offset + payload_length <= context->bytes_written) {
            context->data_write_count++;
            return true;
        }
        set_recoverable_error(context, DENZIC_OTA_V1_ERROR_OFFSET_MISMATCH);
        return false;
    }
    if (offset != context->bytes_written) {
        set_recoverable_error(context, DENZIC_OTA_V1_ERROR_OFFSET_MISMATCH);
        return false;
    }
    if (context->bytes_written > context->expected_size ||
        payload_length > context->expected_size - context->bytes_written) {
        set_error(context, DENZIC_OTA_V1_ERROR_BAD_SIZE);
        return false;
    }
    if (context->driver.write == NULL ||
        !context->driver.write(
            context->driver_context,
            &bytes[DENZIC_OTA_V1_DATA_HEADER_BYTES],
            payload_length)) {
        set_error(context, DENZIC_OTA_V1_ERROR_STORAGE_WRITE);
        return false;
    }
    context->bytes_written += payload_length;
    context->data_write_count++;
    context->last_error = DENZIC_OTA_V1_ERROR_NONE;
    return true;
}

size_t denzic_ota_v1_encode_status(
    const denzic_ota_v1_context_t *context,
    uint8_t *output,
    size_t output_size)
{
    static const uint8_t magic[4] = DENZIC_OTA_V1_MAGIC;
    if (context == NULL || output == NULL || output_size < DENZIC_OTA_V1_STATUS_BYTES) {
        return 0u;
    }
    memset(output, 0, DENZIC_OTA_V1_STATUS_BYTES);
    memcpy(output, magic, sizeof(magic));
    output[4] = DENZIC_OTA_V1_PROTOCOL_VERSION;
    output[5] = context->state;
    output[6] = context->last_error;
    output[7] = context->status_flags;
    write_u32_le(&output[8], context->bytes_written);
    write_u32_le(&output[12], context->expected_size);
    write_u16_le(&output[16], context->chunk_payload_bytes);
    write_u16_le(&output[18], context->window_chunks);
    write_u32_le(&output[20], context->data_write_count);
    return DENZIC_OTA_V1_STATUS_BYTES;
}
