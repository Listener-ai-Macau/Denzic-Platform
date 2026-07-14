#ifndef DENZIC_AUDIO_RECORDING_V1_H
#define DENZIC_AUDIO_RECORDING_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_AUDIO_RECORDING_V1_SAMPLE_RATE_HZ 16000u
#define DENZIC_AUDIO_RECORDING_V1_BITS_PER_SAMPLE 16u
#define DENZIC_AUDIO_RECORDING_V1_CHANNELS 1u
#define DENZIC_AUDIO_RECORDING_V1_FRAME_SAMPLES 160u

typedef enum {
    DENZIC_AUDIO_SOURCE_UNKNOWN = 0,
    DENZIC_AUDIO_SOURCE_SPH0645_I2S_COMPATIBLE = 1,
    DENZIC_AUDIO_SOURCE_SPH0655_PDM_COMPATIBLE = 2,
} denzic_audio_source_family_v1_t;

typedef enum {
    DENZIC_AUDIO_INGRESS_UNKNOWN = 0,
    DENZIC_AUDIO_INGRESS_SAI_I2S_PCM = 1,
    DENZIC_AUDIO_INGRESS_SAI_PDM = 2,
    DENZIC_AUDIO_INGRESS_I2S_PDM = 3,
    DENZIC_AUDIO_INGRESS_DFSDM_PDM = 4,
    DENZIC_AUDIO_INGRESS_SOFTWARE_FIXTURE = 5,
    DENZIC_AUDIO_INGRESS_ESP_I2S_PCM = 6,
    DENZIC_AUDIO_INGRESS_ESP_I2S_PDM = 7,
} denzic_audio_ingress_v1_t;

typedef enum {
    DENZIC_AUDIO_ENCODING_PCM16_MONO = 1,
    DENZIC_AUDIO_ENCODING_PDM_1BIT = 2,
} denzic_audio_encoding_v1_t;

typedef struct {
    denzic_audio_source_family_v1_t source_family;
    denzic_audio_ingress_v1_t ingress;
    denzic_audio_encoding_v1_t encoding;
    uint32_t sample_rate_hz;
    uint16_t bits_per_sample;
    uint8_t channels;
    uint16_t frame_samples;
} denzic_audio_format_v1_t;

typedef struct {
    uint32_t frame_count;
    uint16_t samples_per_frame;
    uint16_t bytes_per_frame;
    uint32_t duration_ms;
    uint32_t dropped_frames;
    uint32_t crc32;
} denzic_audio_frame_batch_v1_t;

typedef enum {
    DENZIC_AUDIO_RECORDING_STATE_IDLE = 0,
    DENZIC_AUDIO_RECORDING_STATE_PENDING_START,
    DENZIC_AUDIO_RECORDING_STATE_RECORDING,
    DENZIC_AUDIO_RECORDING_STATE_STOPPING,
    DENZIC_AUDIO_RECORDING_STATE_CANCELED,
    DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY,
    DENZIC_AUDIO_RECORDING_STATE_TRANSFER_ACTIVE,
    DENZIC_AUDIO_RECORDING_STATE_TRANSFERRED,
    DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY,
} denzic_audio_recording_state_v1_t;

typedef enum {
    DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST = 0,
    DENZIC_AUDIO_RECORDING_EVENT_CAPTURE_READY,
    DENZIC_AUDIO_RECORDING_EVENT_STOP_REQUEST,
    DENZIC_AUDIO_RECORDING_EVENT_CANCEL_REQUEST,
    DENZIC_AUDIO_RECORDING_EVENT_SESSION_FINISHED,
    DENZIC_AUDIO_RECORDING_EVENT_TRANSFER_BEGIN,
    DENZIC_AUDIO_RECORDING_EVENT_TRANSFER_FINISHED,
    DENZIC_AUDIO_RECORDING_EVENT_ERROR_DETECTED,
    DENZIC_AUDIO_RECORDING_EVENT_RECOVERY_COMPLETE,
    DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST,
} denzic_audio_recording_event_v1_t;

typedef enum {
    DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN = 0,
    DENZIC_AUDIO_RECORDING_SOURCE_USER_BUTTON,
    DENZIC_AUDIO_RECORDING_SOURCE_USER_TOUCH,
    DENZIC_AUDIO_RECORDING_SOURCE_HOST_CONTROL,
    DENZIC_AUDIO_RECORDING_SOURCE_SYNTHETIC_FIXTURE,
} denzic_audio_recording_source_v1_t;

typedef enum {
    DENZIC_AUDIO_RECORDING_RESULT_OK = 0,
    DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
    DENZIC_AUDIO_RECORDING_RESULT_CAPTURE_UNAVAILABLE,
    DENZIC_AUDIO_RECORDING_RESULT_BUSY,
    DENZIC_AUDIO_RECORDING_RESULT_BAD_ARGUMENT,
    DENZIC_AUDIO_RECORDING_RESULT_INTERNAL_ERROR,
} denzic_audio_recording_result_v1_t;

typedef enum {
    DENZIC_AUDIO_RECORDING_EFFECT_IGNORE = 0,
    DENZIC_AUDIO_RECORDING_EFFECT_ENTER_PENDING_START,
    DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE,
    DENZIC_AUDIO_RECORDING_EFFECT_STOP_CAPTURE,
    DENZIC_AUDIO_RECORDING_EFFECT_CANCEL_CAPTURE,
    DENZIC_AUDIO_RECORDING_EFFECT_MARK_TRANSFER_READY,
    DENZIC_AUDIO_RECORDING_EFFECT_BEGIN_TRANSFER,
    DENZIC_AUDIO_RECORDING_EFFECT_FINISH_TRANSFER,
    DENZIC_AUDIO_RECORDING_EFFECT_ENTER_ERROR_RECOVERY,
    DENZIC_AUDIO_RECORDING_EFFECT_RECOVER_TO_IDLE,
    DENZIC_AUDIO_RECORDING_EFFECT_REJECT,
} denzic_audio_recording_effect_v1_t;

typedef enum {
    DENZIC_AUDIO_RECORDING_SYNC_UNSYNCED = 0,
    DENZIC_AUDIO_RECORDING_SYNC_TRANSFER_ACTIVE,
    DENZIC_AUDIO_RECORDING_SYNC_TRANSFERRED,
    DENZIC_AUDIO_RECORDING_SYNC_DELETED,
} denzic_audio_recording_sync_v1_t;

typedef struct {
    uint32_t recording_id;
    uint32_t duration_ms;
    uint32_t frame_count;
    uint32_t dropped_frames;
    uint32_t total_length;
    uint32_t content_crc32;
    uint32_t sha256_prefix_words[2];
    uint32_t packet_sequence_next;
    uint32_t byte_offset_ready;
    denzic_audio_recording_sync_v1_t sync_state;
    denzic_audio_format_v1_t sample_format;
} denzic_audio_recording_metadata_v1_t;

typedef struct {
    denzic_audio_recording_state_v1_t state;
    bool capture_ready;
    bool active_capture;
    bool stop_requested;
    bool transfer_window_open;
} denzic_audio_recording_snapshot_v1_t;

typedef struct {
    denzic_audio_recording_effect_v1_t effect;
    denzic_audio_recording_state_v1_t next_state;
    denzic_audio_recording_result_v1_t result;
    const char *detail;
} denzic_audio_recording_decision_v1_t;

typedef struct {
    denzic_audio_recording_state_v1_t state;
    denzic_audio_recording_source_v1_t active_source;
    denzic_audio_recording_source_v1_t pending_source;
    uint32_t next_recording_id;
    bool capture_ready;
    denzic_audio_recording_metadata_v1_t metadata;
    denzic_audio_format_v1_t default_format;
} denzic_audio_recording_context_v1_t;

typedef struct {
    const char *id;
    denzic_audio_recording_state_v1_t state;
    denzic_audio_recording_event_v1_t event;
    bool capture_ready;
    bool active_capture;
    bool stop_requested;
    bool transfer_window_open;
    denzic_audio_recording_effect_v1_t effect;
    denzic_audio_recording_state_v1_t next_state;
    denzic_audio_recording_result_v1_t result;
    const char *detail;
} denzic_audio_recording_transition_case_v1_t;

denzic_audio_format_v1_t denzic_audio_recording_default_format_v1(void);
void denzic_audio_recording_init_v1(
    denzic_audio_recording_context_v1_t *context,
    const denzic_audio_format_v1_t *format);
void denzic_audio_recording_set_capture_ready_v1(
    denzic_audio_recording_context_v1_t *context,
    bool ready);
denzic_audio_recording_decision_v1_t denzic_audio_recording_decide_v1(
    denzic_audio_recording_event_v1_t event,
    const denzic_audio_recording_snapshot_v1_t *snapshot);
denzic_audio_recording_result_v1_t denzic_audio_recording_dispatch_v1(
    denzic_audio_recording_context_v1_t *context,
    denzic_audio_recording_event_v1_t event,
    denzic_audio_recording_source_v1_t source);
denzic_audio_recording_result_v1_t denzic_audio_recording_feed_v1(
    denzic_audio_recording_context_v1_t *context,
    const denzic_audio_frame_batch_v1_t *batch);
const denzic_audio_recording_metadata_v1_t *denzic_audio_recording_metadata_v1(
    const denzic_audio_recording_context_v1_t *context);
size_t denzic_audio_recording_transition_count_v1(void);
const denzic_audio_recording_transition_case_v1_t *
denzic_audio_recording_transition_at_v1(size_t index);
const char *denzic_audio_recording_state_name_v1(
    denzic_audio_recording_state_v1_t state);
const char *denzic_audio_recording_event_name_v1(
    denzic_audio_recording_event_v1_t event);
const char *denzic_audio_recording_effect_name_v1(
    denzic_audio_recording_effect_v1_t effect);

#ifdef __cplusplus
}
#endif

#endif
