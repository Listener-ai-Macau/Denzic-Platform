#include "denzic_audio_v1_generated.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed: %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

int main(void)
{
    denzic_audio_v1_packet_header_t header;
    denzic_audio_v1_packet_header_init(
        &header,
        DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_START,
        0x12345678u,
        7u,
        0u,
        1u,
        0u,
        0u);

    CHECK(sizeof(header) == DENZIC_AUDIO_V1_HEADER_BYTES);
    CHECK(memcmp(header.magic, DENZIC_AUDIO_V1_MAGIC, 4u) == 0);
    CHECK(header.packet_type == DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_START);
    CHECK(header.header_len_le == DENZIC_AUDIO_V1_HEADER_BYTES);
    CHECK(header.session_id_le == 0x12345678u);
    CHECK(header.chunk_index_le == 7u);
    CHECK(header.fragment_index == 0u);
    CHECK(header.fragment_count == 1u);
    CHECK(DENZIC_AUDIO_V1_PCM_BYTES_PER_SECOND == 32000u);
    CHECK(DENZIC_AUDIO_V1_SESSION_STOP_ORIGIN_USER == 0u);
    CHECK(DENZIC_AUDIO_V1_SESSION_STOP_ORIGIN_VOICE_ACTIVATION == 1u);
    CHECK(DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_FLAG_SHIFT == 1u);
    CHECK(DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_FLAG_MASK == 0xfeu);
    CHECK(DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_ABSENT_ENCODED == 0u);
    CHECK(DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_ENCODED_OFFSET == 1u);
    CHECK(DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_MAX_PERCENT == 100u);
    CHECK(
        (DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_FLAG_MASK &
         DENZIC_AUDIO_V1_LOSSLESS_RICE_PACKET_FLAG) == 0u);
    return 0;
}
