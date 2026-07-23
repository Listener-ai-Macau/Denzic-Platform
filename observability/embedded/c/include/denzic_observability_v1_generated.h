/* Generated from observability/protocol/observability_v1.json. Do not edit. */
#ifndef DENZIC_OBSERVABILITY_V1_GENERATED_H
#define DENZIC_OBSERVABILITY_V1_GENERATED_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_OBSERVABILITY_V1_CONTRACT_NAME "denzic_observability_v1"
#define DENZIC_OBSERVABILITY_V1_CONTRACT_VERSION (1u)

typedef enum {
    DENZIC_OBSERVABILITY_V1_EVENT_SOURCE_FIRMWARE = 1,
    DENZIC_OBSERVABILITY_V1_EVENT_SOURCE_TYPE = 2,
    DENZIC_OBSERVABILITY_V1_EVENT_SOURCE_TRANSPORT = 3,
    DENZIC_OBSERVABILITY_V1_EVENT_SOURCE_PROVIDER = 4,
} denzic_observability_v1_event_source_t;

typedef enum {
    DENZIC_OBSERVABILITY_V1_CAPABILITY_BLE = 1,
    DENZIC_OBSERVABILITY_V1_CAPABILITY_AUDIO = 2,
    DENZIC_OBSERVABILITY_V1_CAPABILITY_OTA = 3,
} denzic_observability_v1_capability_t;

typedef enum {
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_UNKNOWN = 0,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_DISCONNECTED = 1,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_ADVERTISING = 2,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_CONNECTING = 3,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_CONNECTED_IDLE = 4,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_RECORDING = 5,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_RECOVERING = 6,
    DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_PAIRED_ELSEWHERE = 7,
} denzic_observability_v1_ble_lifecycle_state_t;

typedef enum {
    DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_STARTED = 1,
    DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_SUCCEEDED = 2,
    DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_CANCELLED = 3,
    DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_FAILED = 4,
    DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_TIMEOUT = 5,
} denzic_observability_v1_command_result_t;

typedef enum {
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_NONE = 0,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_DEVICE = 1,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_TRANSPORT = 2,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_HOST = 3,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_PROVIDER = 4,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_NETWORK = 5,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_PROTOCOL = 6,
    DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_RESOURCE = 7,
} denzic_observability_v1_error_category_t;

typedef enum {
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_NONE = 0,
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_EDGE_TO_RECORD_DISPATCH_MS = 1,
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_BLE_RECOVERY_MS = 2,
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_AUDIO_FIRST_PACKET_MS = 3,
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_PREVIEW_LATENCY_MS = 4,
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_FINAL_TRANSCRIPTION_MS = 5,
    DENZIC_OBSERVABILITY_V1_TIMING_METRIC_OTA_TRANSFER_MS = 6,
} denzic_observability_v1_timing_metric_t;

/* BLE diagnostic log GATT service contract. The data characteristic notifies
 * one chunk per control read: a chunk header (event_count u16 LE,
 * global_offset u32 LE, events_crc32 u32 LE, CRC-32/IEEE over the payload)
 * followed by event_count packed DIAG_LOG_EVENT_WIRE_BYTES events. */
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_SERVICE_UUID_TEXT "710af845-6d9f-6583-0c4d-9e5b3bc3093a"
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_SERVICE_UUID_BYTES 0x3a, 0x09, 0xc3, 0x3b, 0x5b, 0x9e, 0x4d, 0x0c, 0x83, 0x65, 0x9f, 0x6d, 0x45, 0xf8, 0x0a, 0x71
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_CONTROL_UUID_TEXT "710af845-6d9f-6583-0c4d-9e5b3bc3093b"
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_CONTROL_UUID_BYTES 0x3b, 0x09, 0xc3, 0x3b, 0x5b, 0x9e, 0x4d, 0x0c, 0x83, 0x65, 0x9f, 0x6d, 0x45, 0xf8, 0x0a, 0x71
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_DATA_UUID_TEXT "710af845-6d9f-6583-0c4d-9e5b3bc3093c"
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_DATA_UUID_BYTES 0x3c, 0x09, 0xc3, 0x3b, 0x5b, 0x9e, 0x4d, 0x0c, 0x83, 0x65, 0x9f, 0x6d, 0x45, 0xf8, 0x0a, 0x71
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_COUNT_UUID_TEXT "710af845-6d9f-6583-0c4d-9e5b3bc3093d"
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_GATT_COUNT_UUID_BYTES 0x3d, 0x09, 0xc3, 0x3b, 0x5b, 0x9e, 0x4d, 0x0c, 0x83, 0x65, 0x9f, 0x6d, 0x45, 0xf8, 0x0a, 0x71
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_EVENT_WIRE_BYTES (24u)
#define DENZIC_OBSERVABILITY_V1_DIAG_LOG_CHUNK_HEADER_BYTES (10u)

typedef struct {
    uint8_t contract_version;
    uint64_t correlation_id;
    uint32_t event_sequence;
    uint32_t monotonic_ms;
    denzic_observability_v1_event_source_t source;
    denzic_observability_v1_capability_t capability;
    denzic_observability_v1_ble_lifecycle_state_t ble_lifecycle_state;
    denzic_observability_v1_command_result_t command_result;
    denzic_observability_v1_error_category_t error_category;
    denzic_observability_v1_timing_metric_t timing_metric;
    uint32_t timing_value_ms;
} denzic_observability_v1_event_t;

static inline void denzic_observability_v1_event_init(
    denzic_observability_v1_event_t *event,
    uint64_t correlation_id,
    uint32_t event_sequence,
    uint32_t monotonic_ms,
    denzic_observability_v1_event_source_t source,
    denzic_observability_v1_capability_t capability)
{
    if (event == NULL) {
        return;
    }
    event->contract_version = DENZIC_OBSERVABILITY_V1_CONTRACT_VERSION;
    event->correlation_id = correlation_id;
    event->event_sequence = event_sequence;
    event->monotonic_ms = monotonic_ms;
    event->source = source;
    event->capability = capability;
    event->ble_lifecycle_state = DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_UNKNOWN;
    event->command_result = DENZIC_OBSERVABILITY_V1_COMMAND_RESULT_STARTED;
    event->error_category = DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_NONE;
    event->timing_metric = DENZIC_OBSERVABILITY_V1_TIMING_METRIC_NONE;
    event->timing_value_ms = 0u;
}

#ifdef __cplusplus
}
#endif

#endif
