#ifndef DENZIC_OTA_V1_REORDER_H
#define DENZIC_OTA_V1_REORDER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_ota_v1.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Default dual-lane reorder capacity. Product BSS may use fewer slots. */
#ifndef DENZIC_OTA_V1_REORDER_DEFAULT_SLOTS
#define DENZIC_OTA_V1_REORDER_DEFAULT_SLOTS (16u)
#endif

/* Max ATT-sized data packet (header + payload) held in one slot. */
#ifndef DENZIC_OTA_V1_REORDER_MAX_PACKET_BYTES
#define DENZIC_OTA_V1_REORDER_MAX_PACKET_BYTES (512u)
#endif

typedef struct {
    bool valid;
    uint16_t length;
    uint8_t data[DENZIC_OTA_V1_REORDER_MAX_PACKET_BYTES];
} denzic_ota_v1_reorder_slot_t;

typedef struct {
    denzic_ota_v1_reorder_slot_t *slots;
    uint16_t slot_count;
    uint16_t used;
    /* Used to bound how far ahead a future offset may be buffered. */
    uint16_t max_payload_bytes;
} denzic_ota_v1_reorder_t;

void denzic_ota_v1_reorder_init(
    denzic_ota_v1_reorder_t *reorder,
    denzic_ota_v1_reorder_slot_t *slots,
    uint16_t slot_count,
    uint16_t max_payload_bytes);

void denzic_ota_v1_reorder_clear(denzic_ota_v1_reorder_t *reorder);

void denzic_ota_v1_reorder_drain(
    denzic_ota_v1_context_t *context,
    denzic_ota_v1_reorder_t *reorder);

/*
 * Apply a data packet in offset order for dual-lane WWR.
 * Contiguous packets go to denzic_ota_v1_handle_data; future packets are
 * buffered until the gap fills. Returns the same success/failure semantics as
 * denzic_ota_v1_handle_data for applied packets; true when only buffered.
 */
bool denzic_ota_v1_handle_data_ordered(
    denzic_ota_v1_context_t *context,
    denzic_ota_v1_reorder_t *reorder,
    const uint8_t *bytes,
    size_t length);

#ifdef __cplusplus
}
#endif

#endif /* DENZIC_OTA_V1_REORDER_H */
