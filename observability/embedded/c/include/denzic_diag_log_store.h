#ifndef DENZIC_DIAG_LOG_STORE_H
#define DENZIC_DIAG_LOG_STORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_diag_log_store_port.h"

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_DIAG_LOG_EVENT_WIRE_BYTES 24u

/*
 * This layout is the stable retained-log and export wire contract:
 * timestamp u32, source u16, event u8, severity u8, arg1-4 u32.
 */
typedef struct {
    uint32_t timestamp_ms;
    uint16_t source;
    uint8_t event;
    uint8_t severity;
    uint32_t arg1;
    uint32_t arg2;
    uint32_t arg3;
    uint32_t arg4;
} denzic_diag_log_event_t;

/* Compile-time wire size check (typedef form: portable across C99 toolchains). */
typedef char denzic_diag_log_event_wire_size_check[
    (sizeof(denzic_diag_log_event_t) == DENZIC_DIAG_LOG_EVENT_WIRE_BYTES) ? 1 : -1];

typedef struct {
    denzic_diag_log_store_port_t port;
    void *port_context;
    uint16_t *sector_counts;
    uint16_t *sector_sequences;
    uint32_t total_sectors;
    uint16_t write_sector;
    uint16_t write_offset;
    uint16_t sector_sequence;
    uint32_t retained_events;
    uint32_t capacity_events;
    bool initialized;
    bool dumping;
} denzic_diag_log_store_t;

/*
 * storage_size_bytes is the size of the persistent region backing the store.
 * The store must be zero-initialized (or deinitialized) before init.
 * Returns false on invalid arguments or allocation failure.
 */
bool denzic_diag_log_store_init(
    denzic_diag_log_store_t *store,
    const denzic_diag_log_store_port_t *port,
    void *port_context,
    uint32_t storage_size_bytes);

void denzic_diag_log_store_deinit(denzic_diag_log_store_t *store);

/* Stamp an event with the port timestamp and append it to the ring. */
void denzic_diag_log_store_write(
    denzic_diag_log_store_t *store,
    uint16_t source,
    uint8_t event,
    uint8_t severity,
    uint32_t arg1,
    uint32_t arg2,
    uint32_t arg3,
    uint32_t arg4);

/* Append a pre-stamped event (for example one queued at capture time). */
void denzic_diag_log_store_write_event(
    denzic_diag_log_store_t *store,
    const denzic_diag_log_event_t *event);

uint32_t denzic_diag_log_store_count(const denzic_diag_log_store_t *store);
void denzic_diag_log_store_dump(denzic_diag_log_store_t *store);
void denzic_diag_log_store_dump_last(denzic_diag_log_store_t *store, uint32_t count);
void denzic_diag_log_store_dump_last_by_source(
    denzic_diag_log_store_t *store,
    uint32_t count,
    uint16_t source);
void denzic_diag_log_store_clear(denzic_diag_log_store_t *store);
bool denzic_diag_log_store_is_dumping(denzic_diag_log_store_t *store);

/*
 * Read a contiguous range of retained events into a caller-supplied buffer.
 * Returns the number of events actually written. Events are packed 24-byte
 * denzic_diag_log_event_t structs. Thread-safe (acquires the port lock).
 */
uint32_t denzic_diag_log_store_read_range(
    denzic_diag_log_store_t *store,
    uint32_t offset,
    uint32_t limit,
    void *buffer,
    uint32_t buffer_size);

#ifdef __cplusplus
}
#endif

#endif
