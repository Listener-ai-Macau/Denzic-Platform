#ifndef DENZIC_DIAG_LOG_STORE_PORT_H
#define DENZIC_DIAG_LOG_STORE_PORT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    DENZIC_DIAG_LOG_STORE_LOG_INFO = 0,
    DENZIC_DIAG_LOG_STORE_LOG_WARN = 1,
    DENZIC_DIAG_LOG_STORE_LOG_ERROR = 2,
} denzic_diag_log_store_log_level_t;

/*
 * Hardware and OS services the diagnostic log store needs from the product
 * adapter. All callbacks receive the opaque context pointer handed to
 * denzic_diag_log_store_init().
 *
 * Storage model: a byte-addressable persistent region divided into fixed-size
 * sectors. Erased bytes read back as 0xFF. storage_erase() covers
 * [offset, offset + length) and length is always a whole number of sectors.
 *
 * lock/unlock, log, emit_line, pace, and source_name may be NULL:
 * - NULL lock/unlock selects single-threaded operation.
 * - NULL log/emit_line drops status messages and dump output.
 * - NULL pace skips cooperative yields during long scans.
 * - NULL source_name makes dumps print the numeric source id.
 */
typedef struct {
    bool (*storage_read)(void *context, uint32_t offset, uint8_t *buffer, size_t length);
    bool (*storage_write)(void *context, uint32_t offset, const uint8_t *data, size_t length);
    bool (*storage_erase)(void *context, uint32_t offset, size_t length);
    uint32_t (*timestamp_ms)(void *context);
    void (*lock)(void *context);
    void (*unlock)(void *context);
    void (*log)(void *context, denzic_diag_log_store_log_level_t level, const char *message);
    void (*emit_line)(void *context, const char *line);
    void (*pace)(void *context);
    const char *(*source_name)(void *context, uint16_t source);
} denzic_diag_log_store_port_t;

#ifdef __cplusplus
}
#endif

#endif
