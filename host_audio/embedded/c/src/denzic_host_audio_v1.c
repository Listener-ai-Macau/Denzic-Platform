#include "denzic_host_audio_v1.h"

#include <string.h>

static void put_u16_le(uint8_t *dst, uint16_t value)
{
    dst[0] = (uint8_t)(value & 0xFFu);
    dst[1] = (uint8_t)((value >> 8) & 0xFFu);
}

static void put_u32_le(uint8_t *dst, uint32_t value)
{
    dst[0] = (uint8_t)(value & 0xFFu);
    dst[1] = (uint8_t)((value >> 8) & 0xFFu);
    dst[2] = (uint8_t)((value >> 16) & 0xFFu);
    dst[3] = (uint8_t)((value >> 24) & 0xFFu);
}

void denzic_host_audio_v1_wav_header(uint32_t data_size, uint8_t *out)
{
    if (out == NULL) {
        return;
    }
    memset(out, 0, DENZIC_HOST_AUDIO_V1_WAV_HEADER_BYTES);
    memcpy(out + 0, "RIFF", 4);
    put_u32_le(out + 4, data_size + 36u);
    memcpy(out + 8, "WAVE", 4);
    memcpy(out + 12, "fmt ", 4);
    put_u32_le(out + 16, DENZIC_HOST_AUDIO_V1_WAV_FMT_CHUNK_BYTES);
    put_u16_le(out + 20, DENZIC_HOST_AUDIO_V1_WAV_AUDIO_FORMAT_PCM);
    put_u16_le(out + 22, DENZIC_HOST_AUDIO_V1_PCM_CHANNELS);
    put_u32_le(out + 24, DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_RATE_HZ);
    put_u32_le(out + 28, DENZIC_HOST_AUDIO_V1_PCM_BYTE_RATE);
    put_u16_le(out + 32, DENZIC_HOST_AUDIO_V1_PCM_BLOCK_ALIGN);
    put_u16_le(out + 34, DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS);
    memcpy(out + 36, "data", 4);
    put_u32_le(out + 40, data_size);
}
