/* Host audio v1: product-independent PCM/WAV helpers for host-side recording.
 *
 * The constants come from denzic_host_audio_v1_generated.h (generated from
 * host_audio/protocol/host_audio_v1.json). This layer carries no OS, audio
 * device, or network dependency; products inject capture and ASR adapters.
 */
#ifndef DENZIC_HOST_AUDIO_V1_H
#define DENZIC_HOST_AUDIO_V1_H

#include <stdint.h>

#include "denzic_host_audio_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Write the 44-byte RIFF/WAVE PCM header for the contract format
 * (16 kHz / mono / 16-bit little-endian) into out, which must hold at
 * least DENZIC_HOST_AUDIO_V1_WAV_HEADER_BYTES bytes. data_size is the
 * number of PCM payload bytes that follow the header. */
void denzic_host_audio_v1_wav_header(uint32_t data_size, uint8_t *out);

#ifdef __cplusplus
}
#endif

#endif
