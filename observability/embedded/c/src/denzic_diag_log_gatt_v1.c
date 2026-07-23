#include "denzic_diag_log_gatt_v1.h"

void denzic_diag_log_gatt_v1_encode_chunk_header(
    uint8_t *out,
    uint16_t event_count,
    uint32_t global_offset,
    uint32_t events_crc32)
{
    if (out == NULL) {
        return;
    }
    out[0] = (uint8_t)(event_count & 0xffu);
    out[1] = (uint8_t)((event_count >> 8) & 0xffu);
    out[2] = (uint8_t)(global_offset & 0xffu);
    out[3] = (uint8_t)((global_offset >> 8) & 0xffu);
    out[4] = (uint8_t)((global_offset >> 16) & 0xffu);
    out[5] = (uint8_t)((global_offset >> 24) & 0xffu);
    out[6] = (uint8_t)(events_crc32 & 0xffu);
    out[7] = (uint8_t)((events_crc32 >> 8) & 0xffu);
    out[8] = (uint8_t)((events_crc32 >> 16) & 0xffu);
    out[9] = (uint8_t)((events_crc32 >> 24) & 0xffu);
}

denzic_diag_log_gatt_v1_chunk_error_t denzic_diag_log_gatt_v1_decode_chunk(
    const uint8_t *packet,
    size_t packet_len,
    uint16_t *event_count_out,
    uint32_t *global_offset_out,
    uint32_t *events_crc32_out,
    const uint8_t **payload_out,
    size_t *payload_len_out)
{
    const size_t header_bytes = DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES;
    uint16_t event_count;
    size_t payload_len;

    if (packet == NULL || event_count_out == NULL || global_offset_out == NULL
        || events_crc32_out == NULL || payload_out == NULL || payload_len_out == NULL) {
        return DENZIC_DIAG_LOG_GATT_V1_CHUNK_TOO_SHORT;
    }
    if (packet_len < header_bytes) {
        return DENZIC_DIAG_LOG_GATT_V1_CHUNK_TOO_SHORT;
    }

    event_count = (uint16_t)((uint16_t)packet[0] | ((uint16_t)packet[1] << 8));
    if (event_count == 0u) {
        return DENZIC_DIAG_LOG_GATT_V1_CHUNK_EMPTY;
    }

    payload_len = (size_t)event_count * DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES;
    if (packet_len - header_bytes != payload_len) {
        return DENZIC_DIAG_LOG_GATT_V1_CHUNK_PAYLOAD_LENGTH_MISMATCH;
    }

    *event_count_out = event_count;
    *global_offset_out = (uint32_t)packet[2]
        | ((uint32_t)packet[3] << 8)
        | ((uint32_t)packet[4] << 16)
        | ((uint32_t)packet[5] << 24);
    *events_crc32_out = (uint32_t)packet[6]
        | ((uint32_t)packet[7] << 8)
        | ((uint32_t)packet[8] << 16)
        | ((uint32_t)packet[9] << 24);
    *payload_out = packet + header_bytes;
    *payload_len_out = payload_len;
    return DENZIC_DIAG_LOG_GATT_V1_CHUNK_OK;
}
