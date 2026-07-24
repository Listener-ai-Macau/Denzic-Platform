#include <assert.h>
#include <stdint.h>
#include <string.h>

#include "denzic_host_audio_v1.h"

static uint16_t read_u16_le(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t read_u32_le(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16)
        | ((uint32_t)p[3] << 24);
}

int main(void)
{
    uint8_t header[DENZIC_HOST_AUDIO_V1_WAV_HEADER_BYTES];

    assert(DENZIC_HOST_AUDIO_V1_CONTRACT_VERSION == 1u);
    assert(strcmp(DENZIC_HOST_AUDIO_V1_CONTRACT_NAME, "denzic_host_audio_v1") == 0);
    assert(DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_RATE_HZ == 16000u);
    assert(DENZIC_HOST_AUDIO_V1_PCM_CHANNELS == 1u);
    assert(DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS == 16u);
    assert(DENZIC_HOST_AUDIO_V1_PCM_BLOCK_ALIGN == 2u);
    assert(DENZIC_HOST_AUDIO_V1_PCM_BYTE_RATE == 32000u);
    assert(DENZIC_HOST_AUDIO_V1_WAV_HEADER_BYTES == 44u);

    denzic_host_audio_v1_wav_header(8u, header);
    assert(memcmp(header + 0, "RIFF", 4) == 0);
    assert(read_u32_le(header + 4) == 44u);
    assert(memcmp(header + 8, "WAVE", 4) == 0);
    assert(memcmp(header + 12, "fmt ", 4) == 0);
    assert(read_u32_le(header + 16) == 16u);
    assert(read_u16_le(header + 20) == 1u);
    assert(read_u16_le(header + 22) == 1u);
    assert(read_u32_le(header + 24) == 16000u);
    assert(read_u32_le(header + 28) == 32000u);
    assert(read_u16_le(header + 32) == 2u);
    assert(read_u16_le(header + 34) == 16u);
    assert(memcmp(header + 36, "data", 4) == 0);
    assert(read_u32_le(header + 40) == 8u);

    denzic_host_audio_v1_wav_header(0u, header);
    assert(read_u32_le(header + 4) == 36u);
    assert(read_u32_le(header + 40) == 0u);

    denzic_host_audio_v1_wav_header(4u, NULL);
    return 0;
}
