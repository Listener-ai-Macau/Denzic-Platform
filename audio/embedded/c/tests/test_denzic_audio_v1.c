#include "denzic_audio_v1_generated.h"

#include <assert.h>
#include <string.h>

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

    assert(sizeof(header) == DENZIC_AUDIO_V1_HEADER_BYTES);
    assert(memcmp(header.magic, DENZIC_AUDIO_V1_MAGIC, 4u) == 0);
    assert(header.packet_type == DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_START);
    assert(header.header_len_le == DENZIC_AUDIO_V1_HEADER_BYTES);
    assert(header.session_id_le == 0x12345678u);
    assert(header.chunk_index_le == 7u);
    assert(header.fragment_index == 0u);
    assert(header.fragment_count == 1u);
    assert(header.reserved_le == DENZIC_AUDIO_V1_PROTOCOL_VERSION);
    assert(DENZIC_AUDIO_V1_PCM_BYTES_PER_SECOND == 32000u);
    return 0;
}
