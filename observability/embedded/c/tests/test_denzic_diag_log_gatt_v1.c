#include "denzic_diag_log_gatt_v1.h"

#include <assert.h>
#include <string.h>

static void test_encode_decode_round_trip(void)
{
    uint8_t packet[DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES
        + 2u * DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES];
    uint16_t event_count = 0;
    uint32_t global_offset = 0;
    uint32_t events_crc32 = 0;
    const uint8_t *payload = NULL;
    size_t payload_len = 0;

    memset(packet, 0, sizeof(packet));
    denzic_diag_log_gatt_v1_encode_chunk_header(packet, 2u, 70000u, 0xdeadbeefu);
    memset(packet + DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES,
        0x5a,
        2u * DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES);

    /* 32-bit offsets past 65535 must survive the round trip. */
    assert(packet[2] == 0x70u && packet[3] == 0x11u && packet[4] == 0x01u && packet[5] == 0x00u);

    assert(denzic_diag_log_gatt_v1_decode_chunk(
               packet,
               sizeof(packet),
               &event_count,
               &global_offset,
               &events_crc32,
               &payload,
               &payload_len)
        == DENZIC_DIAG_LOG_GATT_V1_CHUNK_OK);
    assert(event_count == 2u);
    assert(global_offset == 70000u);
    assert(events_crc32 == 0xdeadbeefu);
    assert(payload == packet + DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES);
    assert(payload_len == 2u * DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES);
}

static void test_decode_rejects_malformed_chunks(void)
{
    uint8_t packet[DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES
        + DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES];
    uint16_t event_count = 0;
    uint32_t global_offset = 0;
    uint32_t events_crc32 = 0;
    const uint8_t *payload = NULL;
    size_t payload_len = 0;

    memset(packet, 0, sizeof(packet));

    /* Short packet. */
    assert(denzic_diag_log_gatt_v1_decode_chunk(
               packet,
               DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES - 1u,
               &event_count,
               &global_offset,
               &events_crc32,
               &payload,
               &payload_len)
        == DENZIC_DIAG_LOG_GATT_V1_CHUNK_TOO_SHORT);

    /* Empty chunk (event_count == 0). */
    assert(denzic_diag_log_gatt_v1_decode_chunk(
               packet,
               DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES,
               &event_count,
               &global_offset,
               &events_crc32,
               &payload,
               &payload_len)
        == DENZIC_DIAG_LOG_GATT_V1_CHUNK_EMPTY);

    /* Payload length must match event_count exactly. */
    denzic_diag_log_gatt_v1_encode_chunk_header(packet, 2u, 0u, 0u);
    assert(denzic_diag_log_gatt_v1_decode_chunk(
               packet,
               sizeof(packet),
               &event_count,
               &global_offset,
               &events_crc32,
               &payload,
               &payload_len)
        == DENZIC_DIAG_LOG_GATT_V1_CHUNK_PAYLOAD_LENGTH_MISMATCH);
}

int main(void)
{
    test_encode_decode_round_trip();
    test_decode_rejects_malformed_chunks();
    return 0;
}
