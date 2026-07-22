#include "denzic_audio_lossless_v1.h"

#include <string.h>

#define DENZIC_AUDIO_LOSSLESS_V1_MIN_PCM_BYTES 6u

static uint32_t denzic_audio_lossless_v1_zigzag(int32_t residual)
{
    if (residual >= 0) {
        return (uint32_t)residual * 2U;
    }
    return (uint32_t)(-residual) * 2U - 1U;
}

static int16_t denzic_audio_lossless_v1_read_pcm_i16_le(const uint8_t *pcm)
{
    uint16_t raw = (uint16_t)pcm[0] | ((uint16_t)pcm[1] << 8);
    return (int16_t)raw;
}

static int32_t denzic_audio_lossless_v1_residual(
    const uint8_t *pcm,
    uint16_t sample_offset,
    uint8_t predictor)
{
    int32_t current = denzic_audio_lossless_v1_read_pcm_i16_le(pcm + sample_offset * 2U);
    int32_t previous = denzic_audio_lossless_v1_read_pcm_i16_le(pcm + (sample_offset - 1U) * 2U);
    if (predictor == DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER) {
        return current - previous;
    }

    int32_t previous_previous =
        denzic_audio_lossless_v1_read_pcm_i16_le(pcm + (sample_offset - 2U) * 2U);
    if (predictor == DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER) {
        return current - (2 * previous) + previous_previous;
    }

    int32_t previous_third =
        denzic_audio_lossless_v1_read_pcm_i16_le(pcm + (sample_offset - 3U) * 2U);
    if (predictor == DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_THIRD_ORDER) {
        return current - (3 * previous) + (3 * previous_previous) - previous_third;
    }

    int32_t previous_fourth =
        denzic_audio_lossless_v1_read_pcm_i16_le(pcm + (sample_offset - 4U) * 2U);
    return current - (4 * previous) + (6 * previous_previous) -
           (4 * previous_third) + previous_fourth;
}

bool denzic_audio_lossless_v1_encode(
    const uint8_t *pcm,
    uint16_t pcm_bytes,
    uint8_t version,
    uint8_t *encoded,
    uint16_t encoded_capacity,
    uint16_t *encoded_bytes)
{
    if (pcm == NULL || encoded == NULL || encoded_bytes == NULL ||
        pcm_bytes < DENZIC_AUDIO_LOSSLESS_V1_MIN_PCM_BYTES || (pcm_bytes & 1U) != 0 ||
        (version != DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1 &&
         version != DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2 &&
         version != DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3)) {
        return false;
    }

    uint8_t best_k = 0;
    uint8_t best_predictor = DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER;
    uint32_t best_bits = UINT32_MAX;
    uint8_t first_predictor = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1
        ? DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER
        : DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER;
    uint8_t last_predictor = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
        ? DENZIC_AUDIO_V1_LOSSLESS_RICE_V3_MAX_PREDICTOR
        : DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_MAX_PREDICTOR;
    uint16_t sample_count = (uint16_t)(pcm_bytes / 2U);
    uint32_t selected_total_zigzag = 0;
    uint32_t best_predictor_score = UINT32_MAX;

    for (uint8_t predictor = first_predictor; predictor <= last_predictor; ++predictor) {
        uint16_t first_residual_sample = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
            ? predictor
            : DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER;
        if (sample_count <= first_residual_sample) {
            continue;
        }

        uint32_t total_zigzag = 0;
        uint16_t sample_stride = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
            ? (uint16_t)DENZIC_AUDIO_V1_LOSSLESS_RICE_V3_PREDICTOR_SAMPLE_STRIDE
            : 1U;
        for (uint16_t sample_offset = first_residual_sample;
             sample_offset < sample_count;
             sample_offset = (uint16_t)(sample_offset + sample_stride)) {
            total_zigzag += denzic_audio_lossless_v1_zigzag(
                denzic_audio_lossless_v1_residual(pcm, sample_offset, predictor));
        }
        uint32_t header_bits = (version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
                ? (2U + (uint32_t)predictor * 2U)
                : DENZIC_AUDIO_LOSSLESS_V1_HEADER_BYTES) * 8U;
        uint32_t predictor_score = total_zigzag + header_bits;
        if (predictor_score < best_predictor_score) {
            best_predictor_score = predictor_score;
            best_predictor = predictor;
            selected_total_zigzag = total_zigzag;
        }
    }

    if (best_predictor_score == UINT32_MAX) {
        return false;
    }

    uint16_t first_residual_sample = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
        ? best_predictor
        : DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER;
    uint16_t residual_count = (uint16_t)(sample_count - first_residual_sample);
    if (version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3) {
        selected_total_zigzag = 0;
        for (uint16_t sample_offset = first_residual_sample;
             sample_offset < sample_count;
             ++sample_offset) {
            selected_total_zigzag += denzic_audio_lossless_v1_zigzag(
                denzic_audio_lossless_v1_residual(pcm, sample_offset, best_predictor));
        }
    }
    uint8_t first_k = 0;
    uint8_t last_k = DENZIC_AUDIO_LOSSLESS_V1_MAX_K;
    if (version != DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1) {
        uint32_t average_zigzag =
            (selected_total_zigzag + residual_count - 1U) / residual_count;
        uint8_t estimated_k = 0;
        while (average_zigzag > 1U && estimated_k < DENZIC_AUDIO_LOSSLESS_V1_MAX_K) {
            average_zigzag >>= 1U;
            estimated_k++;
        }
        if (version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3) {
            first_k = estimated_k;
            last_k = estimated_k;
        } else {
            first_k = estimated_k > DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_PARAMETER_K_SPAN
                ? (uint8_t)(estimated_k - DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_PARAMETER_K_SPAN)
                : 0;
            last_k = (uint8_t)(estimated_k + DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_PARAMETER_K_SPAN);
            if (last_k > DENZIC_AUDIO_LOSSLESS_V1_MAX_K) {
                last_k = DENZIC_AUDIO_LOSSLESS_V1_MAX_K;
            }
        }
    }

    uint32_t header_bits = (version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
            ? (2U + (uint32_t)best_predictor * 2U)
            : DENZIC_AUDIO_LOSSLESS_V1_HEADER_BYTES) * 8U;
    for (uint8_t rice_k = first_k; rice_k <= last_k; ++rice_k) {
        uint32_t bits = header_bits;
        for (uint16_t sample_offset = first_residual_sample;
             sample_offset < sample_count;
             ++sample_offset) {
            uint32_t zigzag = denzic_audio_lossless_v1_zigzag(
                denzic_audio_lossless_v1_residual(pcm, sample_offset, best_predictor));
            bits += (zigzag >> rice_k) + 1U + rice_k;
        }
        if (bits < best_bits) {
            best_bits = bits;
            best_k = rice_k;
        }
    }

    uint32_t required_bytes = (best_bits + 7U) / 8U;
    if (required_bytes >= pcm_bytes || required_bytes > encoded_capacity) {
        return false;
    }

    memset(encoded, 0, required_bytes);
    encoded[0] = version;
    encoded[1] = version != DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1
        ? (uint8_t)((best_predictor << 4U) | best_k)
        : best_k;
    uint16_t header_bytes = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
        ? (uint16_t)(2U + best_predictor * 2U)
        : (uint16_t)DENZIC_AUDIO_LOSSLESS_V1_HEADER_BYTES;
    uint8_t seed_count = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
        ? best_predictor
        : (uint8_t)DENZIC_AUDIO_V1_LOSSLESS_RICE_V1_SEED_SAMPLES;
    memcpy(encoded + 2U, pcm, seed_count * 2U);

    uint32_t bit_offset = (uint32_t)header_bytes * 8U;
    for (uint16_t sample_offset = first_residual_sample;
         sample_offset < sample_count;
         ++sample_offset) {
        uint32_t zigzag = denzic_audio_lossless_v1_zigzag(
            denzic_audio_lossless_v1_residual(pcm, sample_offset, best_predictor));
        bit_offset += zigzag >> best_k;
        encoded[bit_offset / 8U] |= (uint8_t)(1U << (bit_offset % 8U));
        bit_offset++;
        for (uint8_t bit_index = 0; bit_index < best_k; ++bit_index) {
            if ((zigzag & (1U << bit_index)) != 0) {
                encoded[bit_offset / 8U] |= (uint8_t)(1U << (bit_offset % 8U));
            }
            bit_offset++;
        }
    }

    *encoded_bytes = (uint16_t)required_bytes;
    return true;
}

static bool denzic_audio_lossless_v1_read_bit(
    const uint8_t *bytes,
    size_t byte_count,
    size_t bit_offset,
    bool *bit)
{
    if (bit_offset / 8U >= byte_count) {
        return false;
    }
    *bit = (bytes[bit_offset / 8U] & (uint8_t)(1U << (bit_offset % 8U))) != 0;
    return true;
}

denzic_audio_lossless_v1_decode_result_t denzic_audio_lossless_v1_decode(
    const uint8_t *payload,
    size_t payload_bytes,
    size_t expected_pcm_bytes,
    uint8_t *pcm_out,
    size_t pcm_capacity,
    size_t *pcm_bytes_out)
{
    if (payload == NULL || pcm_out == NULL || pcm_bytes_out == NULL ||
        expected_pcm_bytes < 4U || (expected_pcm_bytes & 1U) != 0 ||
        pcm_capacity < expected_pcm_bytes) {
        return DENZIC_AUDIO_LOSSLESS_V1_DECODE_BAD_ARGUMENT;
    }
    if (payload_bytes < 2U) {
        return DENZIC_AUDIO_LOSSLESS_V1_DECODE_TRUNCATED;
    }

    uint8_t version = payload[0];
    uint8_t control = payload[1];
    uint8_t predictor;
    uint8_t rice_k;
    size_t seed_count;
    switch (version) {
    case DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1:
        predictor = (uint8_t)DENZIC_AUDIO_V1_LOSSLESS_RICE_V1_PREDICTOR;
        rice_k = control;
        seed_count = DENZIC_AUDIO_V1_LOSSLESS_RICE_V1_SEED_SAMPLES;
        break;
    case DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2:
        predictor = (uint8_t)(control >> 4U);
        rice_k = (uint8_t)(control & DENZIC_AUDIO_LOSSLESS_V1_MAX_K);
        seed_count = DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_SEED_SAMPLES;
        break;
    case DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3:
        predictor = (uint8_t)(control >> 4U);
        rice_k = (uint8_t)(control & DENZIC_AUDIO_LOSSLESS_V1_MAX_K);
        seed_count = predictor;
        break;
    default:
        return DENZIC_AUDIO_LOSSLESS_V1_DECODE_UNSUPPORTED_VERSION;
    }
    if (predictor < DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER ||
        predictor > DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FOURTH_ORDER ||
        (version != DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3 &&
         predictor > DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_MAX_PREDICTOR)) {
        return DENZIC_AUDIO_LOSSLESS_V1_DECODE_INVALID_PREDICTOR;
    }
    if (rice_k > DENZIC_AUDIO_LOSSLESS_V1_MAX_K) {
        return DENZIC_AUDIO_LOSSLESS_V1_DECODE_INVALID_PARAMETER;
    }

    size_t header_bytes = version == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3
        ? 2U + seed_count * 2U
        : DENZIC_AUDIO_LOSSLESS_V1_HEADER_BYTES;
    if (payload_bytes < header_bytes || expected_pcm_bytes / 2U < seed_count) {
        return DENZIC_AUDIO_LOSSLESS_V1_DECODE_TRUNCATED;
    }

    size_t sample_total = expected_pcm_bytes / 2U;
    size_t pcm_written = 0;
    int16_t history[DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FOURTH_ORDER];
    size_t history_count = 0;
    for (size_t seed_index = 0; seed_index < seed_count; ++seed_index) {
        size_t offset = 2U + seed_index * 2U;
        int16_t sample = denzic_audio_lossless_v1_read_pcm_i16_le(payload + offset);
        history[history_count++] = sample;
        pcm_out[pcm_written++] = payload[offset];
        pcm_out[pcm_written++] = payload[offset + 1U];
    }

    size_t bit_offset = header_bytes * 8U;
    for (size_t sample_index = seed_count; sample_index < sample_total; ++sample_index) {
        uint32_t quotient = 0;
        for (;;) {
            bool bit = false;
            if (!denzic_audio_lossless_v1_read_bit(payload, payload_bytes, bit_offset, &bit)) {
                return DENZIC_AUDIO_LOSSLESS_V1_DECODE_TRUNCATED;
            }
            bit_offset++;
            if (bit) {
                break;
            }
            if (quotient >= UINT32_MAX) {
                return DENZIC_AUDIO_LOSSLESS_V1_DECODE_RESIDUAL_OVERFLOW;
            }
            quotient++;
            if (quotient > (DENZIC_AUDIO_V1_LOSSLESS_RICE_MAX_ZIGZAG_RESIDUAL >> rice_k)) {
                return DENZIC_AUDIO_LOSSLESS_V1_DECODE_RESIDUAL_OVERFLOW;
            }
        }

        uint32_t remainder = 0;
        for (uint8_t bit_index = 0; bit_index < rice_k; ++bit_index) {
            bool bit = false;
            if (!denzic_audio_lossless_v1_read_bit(payload, payload_bytes, bit_offset, &bit)) {
                return DENZIC_AUDIO_LOSSLESS_V1_DECODE_TRUNCATED;
            }
            bit_offset++;
            if (bit) {
                remainder |= 1UL << bit_index;
            }
        }

        uint32_t zigzag = (quotient << rice_k) | remainder;
        if (zigzag > DENZIC_AUDIO_V1_LOSSLESS_RICE_MAX_ZIGZAG_RESIDUAL) {
            return DENZIC_AUDIO_LOSSLESS_V1_DECODE_RESIDUAL_OVERFLOW;
        }
        int32_t residual = (int32_t)(zigzag >> 1U) ^ -(int32_t)(zigzag & 1U);
        int32_t sample;
        switch (predictor) {
        case DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER:
            sample = (int32_t)history[history_count - 1U] + residual;
            break;
        case DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER:
            sample = 2 * (int32_t)history[history_count - 1U] -
                     (int32_t)history[history_count - 2U] + residual;
            break;
        case DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_THIRD_ORDER:
            sample = 3 * (int32_t)history[history_count - 1U] -
                     3 * (int32_t)history[history_count - 2U] +
                     (int32_t)history[history_count - 3U] + residual;
            break;
        default:
            sample = 4 * (int32_t)history[history_count - 1U] -
                     6 * (int32_t)history[history_count - 2U] +
                     4 * (int32_t)history[history_count - 3U] -
                     (int32_t)history[history_count - 4U] + residual;
            break;
        }
        if (sample < -32768 || sample > 32767) {
            return DENZIC_AUDIO_LOSSLESS_V1_DECODE_SAMPLE_OVERFLOW;
        }

        int16_t sample_i16 = (int16_t)sample;
        if (history_count < DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FOURTH_ORDER) {
            history[history_count++] = sample_i16;
        } else {
            history[0] = history[1];
            history[1] = history[2];
            history[2] = history[3];
            history[3] = sample_i16;
        }
        pcm_out[pcm_written++] = (uint8_t)((uint16_t)sample_i16 & 0xFFu);
        pcm_out[pcm_written++] = (uint8_t)(((uint16_t)sample_i16 >> 8U) & 0xFFu);
    }

    *pcm_bytes_out = pcm_written;
    return DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK;
}
