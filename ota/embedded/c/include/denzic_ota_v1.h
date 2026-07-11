#ifndef DENZIC_OTA_V1_H
#define DENZIC_OTA_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_ota_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    bool (*begin)(void *driver_context, uint32_t image_size);
    bool (*write)(void *driver_context, const uint8_t *data, size_t length);
    bool (*finish)(void *driver_context);
    void (*abort)(void *driver_context);
} denzic_ota_v1_storage_driver_t;

typedef struct {
    denzic_ota_v1_storage_driver_t driver;
    void *driver_context;
    uint32_t expected_size;
    uint32_t bytes_written;
    uint32_t data_write_count;
    uint16_t max_chunk_payload_bytes;
    uint16_t default_window_chunks;
    uint16_t chunk_payload_bytes;
    uint16_t window_chunks;
    uint8_t state;
    uint8_t last_error;
    uint8_t status_flags;
} denzic_ota_v1_context_t;

void denzic_ota_v1_init(
    denzic_ota_v1_context_t *context,
    denzic_ota_v1_storage_driver_t driver,
    void *driver_context,
    uint16_t max_chunk_payload_bytes,
    uint16_t default_window_chunks);

void denzic_ota_v1_reset(denzic_ota_v1_context_t *context);

void denzic_ota_v1_set_status_flags(
    denzic_ota_v1_context_t *context,
    uint8_t status_flags);

bool denzic_ota_v1_handle_control(
    denzic_ota_v1_context_t *context,
    const uint8_t *bytes,
    size_t length);

bool denzic_ota_v1_handle_data(
    denzic_ota_v1_context_t *context,
    const uint8_t *bytes,
    size_t length);

size_t denzic_ota_v1_encode_status(
    const denzic_ota_v1_context_t *context,
    uint8_t *output,
    size_t output_size);

#ifdef __cplusplus
}
#endif

#endif
