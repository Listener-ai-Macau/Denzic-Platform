#ifndef DENZIC_AUDIO_LOSSLESS_V1_H
#define DENZIC_AUDIO_LOSSLESS_V1_H

/* Lossless predictive Rice codec for VKA1 audio-data payloads.
 *
 * Wire contract (audio/protocol/audio_v1.json, "lossless_rice"):
 * - byte 0: codec version (V1/V2/V3)
 * - byte 1: control byte
 *     V1: rice_k; predictor is fixed second order
 *     V2: high nibble = predictor order (1..2), low nibble = rice_k
 *     V3: high nibble = predictor order (1..4, also the seed count),
 *         low nibble = rice_k
 * - seed samples: little-endian i16; two seeds for V1/V2, predictor-order
 *   seeds for V3 (header is 6 bytes for V1/V2, 2 + 2 * predictor for V3)
 * - residuals: zigzag mapped, unary quotient (zero bits terminated by a one
 *   bit) followed by k remainder bits; bits are packed LSB first within each
 *   byte
 *
 * The codec is pure C99 with no OS, heap, or I/O dependency; callers own all
 * buffers. The encoder refuses input that would not shrink so transports can
 * fall back to raw PCM without a wire-format change. */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_audio_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_AUDIO_LOSSLESS_V1_FLAG DENZIC_AUDIO_V1_LOSSLESS_RICE_PACKET_FLAG
#define DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1 DENZIC_AUDIO_V1_LOSSLESS_RICE_VERSION_V1
#define DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2 DENZIC_AUDIO_V1_LOSSLESS_RICE_VERSION_V2
#define DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3 DENZIC_AUDIO_V1_LOSSLESS_RICE_VERSION_V3
#define DENZIC_AUDIO_LOSSLESS_V1_HEADER_BYTES DENZIC_AUDIO_V1_LOSSLESS_RICE_HEADER_BYTES
#define DENZIC_AUDIO_LOSSLESS_V1_MAX_K DENZIC_AUDIO_V1_LOSSLESS_RICE_MAX_K
#define DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER \
    DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_FIRST_ORDER
#define DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER \
    DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_SECOND_ORDER
#define DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_THIRD_ORDER \
    DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_THIRD_ORDER
#define DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FOURTH_ORDER \
    DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_FOURTH_ORDER

typedef enum {
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK = 0,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_BAD_ARGUMENT,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_UNSUPPORTED_VERSION,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_INVALID_PREDICTOR,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_INVALID_PARAMETER,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_TRUNCATED,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_RESIDUAL_OVERFLOW,
    DENZIC_AUDIO_LOSSLESS_V1_DECODE_SAMPLE_OVERFLOW,
} denzic_audio_lossless_v1_decode_result_t;

/* Encodes pcm_bytes of little-endian i16 PCM with the selected codec version
 * (V1/V2/V3). Returns false when the arguments are invalid, when the encoded
 * frame would not be smaller than the raw PCM, or when it does not fit in
 * encoded_capacity. On success *encoded_bytes carries the exact frame size. */
bool denzic_audio_lossless_v1_encode(
    const uint8_t *pcm,
    uint16_t pcm_bytes,
    uint8_t version,
    uint8_t *encoded,
    uint16_t encoded_capacity,
    uint16_t *encoded_bytes);

/* Decodes one lossless Rice payload back to exactly expected_pcm_bytes of
 * little-endian i16 PCM. pcm_capacity must be at least expected_pcm_bytes.
 * On DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK, *pcm_bytes_out equals
 * expected_pcm_bytes. */
denzic_audio_lossless_v1_decode_result_t denzic_audio_lossless_v1_decode(
    const uint8_t *payload,
    size_t payload_bytes,
    size_t expected_pcm_bytes,
    uint8_t *pcm_out,
    size_t pcm_capacity,
    size_t *pcm_bytes_out);

#ifdef __cplusplus
}
#endif

#endif
