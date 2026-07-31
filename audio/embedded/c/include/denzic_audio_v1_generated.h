/* Generated from audio/protocol/audio_v1.json. Do not edit. */
#ifndef DENZIC_AUDIO_V1_GENERATED_H
#define DENZIC_AUDIO_V1_GENERATED_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_AUDIO_V1_PROTOCOL_NAME "denzic_audio_v1"
#define DENZIC_AUDIO_V1_MAGIC "VKA1"
#define DENZIC_AUDIO_V1_MAGIC_U32 (0x31414b56u)
#define DENZIC_AUDIO_V1_PROTOCOL_VERSION (1u)
#define DENZIC_AUDIO_V1_HEADER_BYTES (20u)
#define DENZIC_AUDIO_V1_PCM_SAMPLE_RATE_HZ (16000u)
#define DENZIC_AUDIO_V1_PCM_CHANNELS (1u)
#define DENZIC_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS (16u)
#define DENZIC_AUDIO_V1_PCM_BYTES_PER_SECOND (DENZIC_AUDIO_V1_PCM_SAMPLE_RATE_HZ * DENZIC_AUDIO_V1_PCM_CHANNELS * (DENZIC_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS / 8u))

typedef enum {
    DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_START = 1,
    DENZIC_AUDIO_V1_PACKET_TYPE_AUDIO_DATA = 2,
    DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_STOP = 3,
    DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_CANCEL = 4,
    DENZIC_AUDIO_V1_PACKET_TYPE_SESSION_ERROR = 5,
    DENZIC_AUDIO_V1_PACKET_TYPE_AUDIO_CHUNK = DENZIC_AUDIO_V1_PACKET_TYPE_AUDIO_DATA,
} denzic_audio_v1_packet_type_t;

typedef enum {
    DENZIC_AUDIO_V1_SESSION_START_ORIGIN_USER = 0,
    DENZIC_AUDIO_V1_SESSION_START_ORIGIN_VOICE_ACTIVATION = 1,
} denzic_audio_v1_session_start_origin_t;

typedef enum {
    DENZIC_AUDIO_V1_SESSION_STOP_ORIGIN_USER = 0,
    DENZIC_AUDIO_V1_SESSION_STOP_ORIGIN_VOICE_ACTIVATION = 1,
} denzic_audio_v1_session_stop_origin_t;

typedef enum {
    DENZIC_AUDIO_V1_SESSION_ERROR_NONE = 0,
    DENZIC_AUDIO_V1_SESSION_ERROR_QUEUE_FULL = 1,
    DENZIC_AUDIO_V1_SESSION_ERROR_NOTIFY_TIMEOUT = 2,
    DENZIC_AUDIO_V1_SESSION_ERROR_LINK_LOST = 3,
    DENZIC_AUDIO_V1_SESSION_ERROR_SEQUENCE_OVERFLOW = 4,
    DENZIC_AUDIO_V1_SESSION_ERROR_INVALID_STATE = 5,
    DENZIC_AUDIO_V1_SESSION_ERROR_NO_MEMORY = 6,
    DENZIC_AUDIO_V1_SESSION_ERROR_PACKET_TOO_LARGE = 7,
    DENZIC_AUDIO_V1_SESSION_ERROR_TRANSPORT = 8,
} denzic_audio_v1_session_error_t;

#define DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_FLAG_SHIFT (1u)
#define DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_FLAG_MASK (0xfeu)
#define DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_ABSENT_ENCODED (0u)
#define DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_ENCODED_OFFSET (1u)
#define DENZIC_AUDIO_V1_RAW_INPUT_LEVEL_MAX_PERCENT (100u)

#define DENZIC_AUDIO_V1_LOSSLESS_RICE_PACKET_FLAG (0x01u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_HEADER_BYTES (6u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_MAX_K (15u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_MAX_ZIGZAG_RESIDUAL (1048560u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_VERSION_V1 (1u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_VERSION_V2 (2u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_VERSION_V3 (3u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_FIRST_ORDER (1u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_SECOND_ORDER (2u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_THIRD_ORDER (3u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_PREDICTOR_FOURTH_ORDER (4u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V1_PREDICTOR (2u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V1_SEED_SAMPLES (2u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_MAX_PREDICTOR (2u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_SEED_SAMPLES (2u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_PARAMETER_K_SPAN (1u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V3_MAX_PREDICTOR (4u)
#define DENZIC_AUDIO_V1_LOSSLESS_RICE_V3_PREDICTOR_SAMPLE_STRIDE (16u)

#if defined(_MSC_VER)
#pragma pack(push, 1)
#define DENZIC_AUDIO_V1_PACKED
#else
#define DENZIC_AUDIO_V1_PACKED __attribute__((packed))
#endif
typedef struct DENZIC_AUDIO_V1_PACKED {
    uint8_t magic[4];
    uint8_t packet_type;
    uint8_t flags;
    uint16_t header_len_le;
    uint32_t session_id_le;
    uint16_t chunk_index_le;
    uint8_t fragment_index;
    uint8_t fragment_count;
    uint16_t payload_len_le;
    uint16_t chunk_pcm_bytes_le;
} denzic_audio_v1_packet_header_t;
#if defined(_MSC_VER)
#pragma pack(pop)
#endif

static inline void denzic_audio_v1_packet_header_init(
    denzic_audio_v1_packet_header_t *header,
    denzic_audio_v1_packet_type_t packet_type,
    uint32_t session_id,
    uint16_t chunk_index,
    uint8_t fragment_index,
    uint8_t fragment_count,
    uint16_t payload_len,
    uint16_t chunk_pcm_bytes)
{
    if (header == NULL) {
        return;
    }
    header->magic[0] = 'V';
    header->magic[1] = 'K';
    header->magic[2] = 'A';
    header->magic[3] = '1';
    header->packet_type = (uint8_t)packet_type;
    header->flags = 0u;
    header->header_len_le = DENZIC_AUDIO_V1_HEADER_BYTES;
    header->session_id_le = session_id;
    header->chunk_index_le = chunk_index;
    header->fragment_index = fragment_index;
    header->fragment_count = fragment_count;
    header->payload_len_le = payload_len;
    header->chunk_pcm_bytes_le = chunk_pcm_bytes;
}

#ifdef __cplusplus
}
#endif

#endif
