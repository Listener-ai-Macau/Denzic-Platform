#ifndef DENZIC_DIAG_LOG_GATT_V1_H
#define DENZIC_DIAG_LOG_GATT_V1_H

#include <stddef.h>
#include <stdint.h>

#include "denzic_observability_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Chunk codec for the BLE diagnostic log GATT pull contract
 * (ble_diag_log_gatt in observability_v1.json). Pure functions with no OS,
 * allocation, or CRC dependencies: the adapter computes the CRC-32/IEEE over
 * the packed event payload with its platform CRC primitive.
 *
 * Chunk wire layout (DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES):
 *   [event_count u16 LE][global_offset u32 LE][events_crc32 u32 LE]
 * followed by event_count packed events of
 * DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES each.
 * global_offset is the absolute retained-log offset of the first event in the
 * chunk; it is 32-bit so exports do not truncate after 65535 events.
 */

/* Writes the chunk header into out (must hold CHUNK_HEADER_BYTES bytes). */
void denzic_diag_log_gatt_v1_encode_chunk_header(
    uint8_t *out,
    uint16_t event_count,
    uint32_t global_offset,
    uint32_t events_crc32);

typedef enum {
    DENZIC_DIAG_LOG_GATT_V1_CHUNK_OK = 0,
    DENZIC_DIAG_LOG_GATT_V1_CHUNK_TOO_SHORT,
    DENZIC_DIAG_LOG_GATT_V1_CHUNK_EMPTY,
    DENZIC_DIAG_LOG_GATT_V1_CHUNK_PAYLOAD_LENGTH_MISMATCH
} denzic_diag_log_gatt_v1_chunk_error_t;

/*
 * Decodes one chunk notification. On OK all outputs are written and
 * *payload_out points into packet with *payload_len_out ==
 * event_count * DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES.
 * The CRC is returned for the caller to verify; decoding does not check it.
 */
denzic_diag_log_gatt_v1_chunk_error_t denzic_diag_log_gatt_v1_decode_chunk(
    const uint8_t *packet,
    size_t packet_len,
    uint16_t *event_count_out,
    uint32_t *global_offset_out,
    uint32_t *events_crc32_out,
    const uint8_t **payload_out,
    size_t *payload_len_out);

#ifdef __cplusplus
}
#endif

#endif
