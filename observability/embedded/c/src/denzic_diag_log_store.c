#include "denzic_diag_log_store.h"

#include <inttypes.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define DENZIC_DIAG_LOG_STORE_SECTOR_SIZE 4096u
#define DENZIC_DIAG_LOG_STORE_MAGIC 0xD1A90001u
#define DENZIC_DIAG_LOG_STORE_DUMP_PACE_EVENTS 64u
#define DENZIC_DIAG_LOG_STORE_SNAPSHOT_READ_BATCH_EVENTS 32u
#define DENZIC_DIAG_LOG_SOURCE_LAST_MAX_SCANNED_EVENTS 512u
#define DENZIC_DIAG_LOG_STORE_SECTOR_HEADER_SIZE 16u

typedef struct {
    uint32_t magic;
    uint16_t sequence;
    uint16_t count;
    uint32_t first_timestamp;
    uint32_t reserved;
} denzic_diag_log_sector_header_t;

typedef char denzic_diag_log_sector_header_size_check[
    (sizeof(denzic_diag_log_sector_header_t) == DENZIC_DIAG_LOG_STORE_SECTOR_HEADER_SIZE) ? 1 : -1];

#define DENZIC_DIAG_LOG_EVENTS_PER_SECTOR \
    ((DENZIC_DIAG_LOG_STORE_SECTOR_SIZE - DENZIC_DIAG_LOG_STORE_SECTOR_HEADER_SIZE) / DENZIC_DIAG_LOG_EVENT_WIRE_BYTES)

typedef struct {
    uint16_t sector;
    uint16_t index;
    uint16_t sequence;
} store_read_slot_t;

static void store_lock(denzic_diag_log_store_t *store)
{
    if (store->port.lock != NULL) {
        store->port.lock(store->port_context);
    }
}

static void store_unlock(denzic_diag_log_store_t *store)
{
    if (store->port.unlock != NULL) {
        store->port.unlock(store->port_context);
    }
}

static void store_pace(denzic_diag_log_store_t *store)
{
    if (store->port.pace != NULL) {
        store->port.pace(store->port_context);
    }
}

static void store_emit(denzic_diag_log_store_t *store, const char *line)
{
    if (store->port.emit_line != NULL) {
        store->port.emit_line(store->port_context, line);
    }
}

static void store_log(denzic_diag_log_store_t *store,
                      denzic_diag_log_store_log_level_t level,
                      const char *format,
                      ...)
{
    if (store->port.log == NULL) {
        return;
    }

    char message[192];
    va_list args;
    va_start(args, format);
    vsnprintf(message, sizeof(message), format, args);
    va_end(args);
    message[sizeof(message) - 1u] = '\0';
    store->port.log(store->port_context, level, message);
}

static void store_set_dumping(denzic_diag_log_store_t *store, bool dumping)
{
    store_lock(store);
    store->dumping = dumping;
    store_unlock(store);
}

static uint32_t store_timestamp_ms(denzic_diag_log_store_t *store)
{
    return store->port.timestamp_ms(store->port_context);
}

static bool read_sector_header(denzic_diag_log_store_t *store,
                               uint16_t sector,
                               denzic_diag_log_sector_header_t *header)
{
    uint32_t offset = (uint32_t)sector * DENZIC_DIAG_LOG_STORE_SECTOR_SIZE;
    return store->port.storage_read(store->port_context, offset,
                                    (uint8_t *)header, sizeof(*header));
}

static bool write_sector_header(denzic_diag_log_store_t *store,
                                uint16_t sector,
                                const denzic_diag_log_sector_header_t *header)
{
    uint32_t offset = (uint32_t)sector * DENZIC_DIAG_LOG_STORE_SECTOR_SIZE;
    return store->port.storage_write(store->port_context, offset,
                                     (const uint8_t *)header, sizeof(*header));
}

static bool erase_sector(denzic_diag_log_store_t *store, uint16_t sector)
{
    uint32_t offset = (uint32_t)sector * DENZIC_DIAG_LOG_STORE_SECTOR_SIZE;
    return store->port.storage_erase(store->port_context, offset,
                                     DENZIC_DIAG_LOG_STORE_SECTOR_SIZE);
}

static bool read_event(denzic_diag_log_store_t *store,
                       uint16_t sector,
                       uint16_t index,
                       denzic_diag_log_event_t *event)
{
    uint32_t offset = (uint32_t)sector * DENZIC_DIAG_LOG_STORE_SECTOR_SIZE
                    + DENZIC_DIAG_LOG_STORE_SECTOR_HEADER_SIZE
                    + (uint32_t)index * DENZIC_DIAG_LOG_EVENT_WIRE_BYTES;
    return store->port.storage_read(store->port_context, offset,
                                    (uint8_t *)event, DENZIC_DIAG_LOG_EVENT_WIRE_BYTES);
}

static bool event_is_erased(const denzic_diag_log_event_t *event)
{
    const uint8_t *bytes = (const uint8_t *)event;
    for (size_t i = 0; i < sizeof(*event); i++) {
        if (bytes[i] != 0xFFu) {
            return false;
        }
    }
    return true;
}

static uint16_t retained_count_from_sector(denzic_diag_log_store_t *store,
                                           uint16_t sector,
                                           const denzic_diag_log_sector_header_t *header)
{
    if (header == NULL || header->magic != DENZIC_DIAG_LOG_STORE_MAGIC) {
        return 0;
    }

    denzic_diag_log_event_t event;
    const uint16_t last_index = (uint16_t)(DENZIC_DIAG_LOG_EVENTS_PER_SECTOR - 1U);
    if (!read_event(store, sector, last_index, &event)) {
        return 0;
    }
    if (!event_is_erased(&event)) {
        return (uint16_t)DENZIC_DIAG_LOG_EVENTS_PER_SECTOR;
    }

    if (!read_event(store, sector, 0, &event) || event_is_erased(&event)) {
        return 0;
    }

    uint16_t low = 1U;
    uint16_t high = last_index;
    while (low < high) {
        uint16_t mid = (uint16_t)(low + ((high - low) / 2U));
        if (!read_event(store, sector, mid, &event) || event_is_erased(&event)) {
            high = mid;
        } else {
            low = (uint16_t)(mid + 1U);
        }
    }
    return low;
}

static uint16_t cached_retained_count_from_sector(denzic_diag_log_store_t *store,
                                                  uint16_t sector,
                                                  const denzic_diag_log_sector_header_t *header)
{
    if (header == NULL || header->magic != DENZIC_DIAG_LOG_STORE_MAGIC) {
        return 0;
    }
    if (store->sector_counts != NULL && sector < store->total_sectors) {
        return store->sector_counts[sector];
    }
    return retained_count_from_sector(store, sector, header);
}

static void pace_dump_output(denzic_diag_log_store_t *store, uint32_t event_counter)
{
    if (event_counter == 0) {
        return;
    }

    if ((event_counter % DENZIC_DIAG_LOG_STORE_DUMP_PACE_EVENTS) != 0) {
        return;
    }

    store_pace(store);
}

static void pace_flash_snapshot(denzic_diag_log_store_t *store)
{
    store_pace(store);
}

static void find_write_position(denzic_diag_log_store_t *store)
{
    uint16_t best_seq = 0;
    uint16_t best_sector = 0;
    uint16_t best_count = 0;

    store->retained_events = 0;
    if (store->sector_counts != NULL) {
        memset(store->sector_counts, 0, store->total_sectors * sizeof(store->sector_counts[0]));
    }
    if (store->sector_sequences != NULL) {
        memset(store->sector_sequences, 0, store->total_sectors * sizeof(store->sector_sequences[0]));
    }

    for (uint32_t i = 0; i < store->total_sectors; i++) {
        denzic_diag_log_sector_header_t header;
        if (!read_sector_header(store, (uint16_t)i, &header)
            || header.magic != DENZIC_DIAG_LOG_STORE_MAGIC) {
            continue;
        }

        uint16_t count = retained_count_from_sector(store, (uint16_t)i, &header);
        store->retained_events += count;
        if (store->sector_counts != NULL) {
            store->sector_counts[i] = count;
        }
        if (store->sector_sequences != NULL) {
            store->sector_sequences[i] = header.sequence;
        }

        if (header.sequence > best_seq || best_seq == 0) {
            best_seq = header.sequence;
            best_sector = (uint16_t)i;
            best_count = count;
        }
    }

    if (store->retained_events > store->capacity_events) {
        store->retained_events = store->capacity_events;
    }

    if (best_seq == 0) {
        store->write_sector = 0;
        store->write_offset = 0;
        store->sector_sequence = 1;
        return;
    }

    store->sector_sequence = best_seq + 1;
    store->write_sector = best_sector;
    store->write_offset = best_count;

    if (store->write_offset >= DENZIC_DIAG_LOG_EVENTS_PER_SECTOR) {
        store->write_sector = (best_sector + 1) % (uint16_t)store->total_sectors;
        store->write_offset = 0;
    }
}

static const char *severity_name(uint8_t severity)
{
    switch (severity) {
    case 0: return "INFO";
    case 1: return "WARN";
    case 2: return "ERROR";
    default: return "?";
    }
}

static const char *store_source_name(denzic_diag_log_store_t *store, uint16_t source)
{
    if (store->port.source_name == NULL) {
        return NULL;
    }
    return store->port.source_name(store->port_context, source);
}

static void dump_event_json(denzic_diag_log_store_t *store, const denzic_diag_log_event_t *event)
{
    char line[192];
    const char *source = store_source_name(store, event->source);
    if (source != NULL) {
        snprintf(line, sizeof(line),
                 "{\"t\":%" PRIu32 ",\"src\":\"%s\",\"evt\":%u,\"sev\":\"%s\","
                 "\"a1\":%" PRIu32 ",\"a2\":%" PRIu32 ",\"a3\":%" PRIu32 ",\"a4\":%" PRIu32 "}",
                 event->timestamp_ms, source, (unsigned)event->event,
                 severity_name(event->severity),
                 event->arg1, event->arg2, event->arg3, event->arg4);
    } else {
        snprintf(line, sizeof(line),
                 "{\"t\":%" PRIu32 ",\"src\":%u,\"evt\":%u,\"sev\":\"%s\","
                 "\"a1\":%" PRIu32 ",\"a2\":%" PRIu32 ",\"a3\":%" PRIu32 ",\"a4\":%" PRIu32 "}",
                 event->timestamp_ms, (unsigned)event->source, (unsigned)event->event,
                 severity_name(event->severity),
                 event->arg1, event->arg2, event->arg3, event->arg4);
    }
    line[sizeof(line) - 1u] = '\0';
    store_emit(store, line);
}

/* Find the sector with the minimum sequence for ordered iteration */
static bool find_oldest_sector(denzic_diag_log_store_t *store, uint16_t *out_start_sector)
{
    if (out_start_sector == NULL) {
        return false;
    }

    uint16_t min_seq = UINT16_MAX;
    uint16_t start_sec = 0;
    bool found = false;

    for (uint32_t sec = 0; sec < store->total_sectors; sec++) {
        denzic_diag_log_sector_header_t header;
        uint16_t count = 0;

        store_lock(store);
        if (read_sector_header(store, (uint16_t)sec, &header)) {
            count = cached_retained_count_from_sector(store, (uint16_t)sec, &header);
        }
        store_unlock(store);

        if (count > 0 && (!found || header.sequence < min_seq)) {
            min_seq = header.sequence;
            start_sec = (uint16_t)sec;
            found = true;
        }
    }

    *out_start_sector = start_sec;
    return found;
}

static uint16_t snapshot_sector_events(denzic_diag_log_store_t *store,
                                       uint16_t sector,
                                       denzic_diag_log_event_t *events,
                                       uint16_t max_events)
{
    if (events == NULL || max_events == 0) {
        return 0;
    }

    uint16_t count = 0;
    store_lock(store);
    denzic_diag_log_sector_header_t header;
    if (read_sector_header(store, sector, &header)) {
        count = cached_retained_count_from_sector(store, sector, &header);
        if (count > max_events) {
            count = max_events;
        }
    }
    store_unlock(store);

    uint16_t read_count = 0;
    while (read_count < count) {
        uint16_t batch_end = (uint16_t)(read_count + DENZIC_DIAG_LOG_STORE_SNAPSHOT_READ_BATCH_EVENTS);
        if (batch_end > count) {
            batch_end = count;
        }

        store_lock(store);
        bool ok = true;
        for (uint16_t idx = read_count; idx < batch_end; idx++) {
            ok = read_event(store, sector, idx, &events[idx]);
            if (!ok) {
                break;
            }
        }
        store_unlock(store);
        if (!ok) {
            return read_count;
        }

        read_count = batch_end;
        if (read_count < count) {
            pace_flash_snapshot(store);
        }
    }

    return read_count;
}

static uint32_t capped_last_count(denzic_diag_log_store_t *store, uint32_t count)
{
    uint32_t capacity = 0;
    store_lock(store);
    capacity = store->capacity_events;
    store_unlock(store);

    if (capacity > 0 && count > capacity) {
        return capacity;
    }
    return count;
}

bool denzic_diag_log_store_init(denzic_diag_log_store_t *store,
                                const denzic_diag_log_store_port_t *port,
                                void *port_context,
                                uint32_t storage_size_bytes)
{
    if (store == NULL || port == NULL
        || port->storage_read == NULL || port->storage_write == NULL
        || port->storage_erase == NULL || port->timestamp_ms == NULL) {
        return false;
    }

    uint32_t total_sectors = storage_size_bytes / DENZIC_DIAG_LOG_STORE_SECTOR_SIZE;
    if (total_sectors == 0 || total_sectors > UINT16_MAX) {
        return false;
    }

    memset(store, 0, sizeof(*store));
    store->port = *port;
    store->port_context = port_context;
    store->total_sectors = total_sectors;
    store->capacity_events = total_sectors * DENZIC_DIAG_LOG_EVENTS_PER_SECTOR;

    store->sector_counts = (uint16_t *)calloc(total_sectors, sizeof(uint16_t));
    store->sector_sequences = (uint16_t *)calloc(total_sectors, sizeof(uint16_t));
    if (store->sector_counts == NULL || store->sector_sequences == NULL) {
        free(store->sector_counts);
        free(store->sector_sequences);
        store->sector_counts = NULL;
        store->sector_sequences = NULL;
        return false;
    }

    find_write_position(store);

    store->initialized = true;
    return true;
}

void denzic_diag_log_store_deinit(denzic_diag_log_store_t *store)
{
    if (store == NULL) {
        return;
    }
    free(store->sector_counts);
    free(store->sector_sequences);
    store->sector_counts = NULL;
    store->sector_sequences = NULL;
    store->initialized = false;
}

void denzic_diag_log_store_write_event(denzic_diag_log_store_t *store,
                                       const denzic_diag_log_event_t *event)
{
    if (store == NULL || event == NULL || !store->initialized) {
        return;
    }

    store_lock(store);

    if (store->write_offset == 0) {
        denzic_diag_log_sector_header_t old_header;
        if (read_sector_header(store, store->write_sector, &old_header)) {
            uint16_t old_count =
                store->sector_counts != NULL
                    ? store->sector_counts[store->write_sector]
                    : retained_count_from_sector(store, store->write_sector, &old_header);
            store->retained_events = old_count > store->retained_events
                ? 0
                : store->retained_events - old_count;
        }

        if (!erase_sector(store, store->write_sector)) {
            store_log(store, DENZIC_DIAG_LOG_STORE_LOG_WARN,
                      "erase sector %u failed", (unsigned)store->write_sector);
            store_unlock(store);
            return;
        }
        if (store->sector_counts != NULL) {
            store->sector_counts[store->write_sector] = 0;
        }
        if (store->sector_sequences != NULL) {
            store->sector_sequences[store->write_sector] = 0;
        }

        denzic_diag_log_sector_header_t header = {
            .magic = DENZIC_DIAG_LOG_STORE_MAGIC,
            .sequence = store->sector_sequence++,
            .count = 0,
            .first_timestamp = event->timestamp_ms,
            .reserved = 0,
        };
        if (!write_sector_header(store, store->write_sector, &header)) {
            store_unlock(store);
            return;
        }
        if (store->sector_sequences != NULL) {
            store->sector_sequences[store->write_sector] = header.sequence;
        }
    }

    uint16_t written_sector = store->write_sector;
    uint32_t offset = (uint32_t)store->write_sector * DENZIC_DIAG_LOG_STORE_SECTOR_SIZE
                    + DENZIC_DIAG_LOG_STORE_SECTOR_HEADER_SIZE
                    + (uint32_t)store->write_offset * DENZIC_DIAG_LOG_EVENT_WIRE_BYTES;
    if (!store->port.storage_write(store->port_context, offset,
                                   (const uint8_t *)event, DENZIC_DIAG_LOG_EVENT_WIRE_BYTES)) {
        store_log(store, DENZIC_DIAG_LOG_STORE_LOG_WARN, "write event failed");
        store_unlock(store);
        return;
    }

    store->write_offset++;
    if (store->sector_counts != NULL && store->sector_counts[written_sector] < store->write_offset) {
        store->sector_counts[written_sector] = store->write_offset;
    }
    if (store->retained_events < store->capacity_events) {
        store->retained_events++;
    }

    if (store->write_offset >= DENZIC_DIAG_LOG_EVENTS_PER_SECTOR) {
        store->write_sector = (store->write_sector + 1) % (uint16_t)store->total_sectors;
        store->write_offset = 0;
    }

    store_unlock(store);
}

void denzic_diag_log_store_write(denzic_diag_log_store_t *store,
                                 uint16_t source,
                                 uint8_t event,
                                 uint8_t severity,
                                 uint32_t arg1,
                                 uint32_t arg2,
                                 uint32_t arg3,
                                 uint32_t arg4)
{
    if (store == NULL || !store->initialized) {
        return;
    }

    const denzic_diag_log_event_t stamped = {
        .timestamp_ms = store_timestamp_ms(store),
        .source = source,
        .event = event,
        .severity = severity,
        .arg1 = arg1,
        .arg2 = arg2,
        .arg3 = arg3,
        .arg4 = arg4,
    };
    denzic_diag_log_store_write_event(store, &stamped);
}

uint32_t denzic_diag_log_store_count(const denzic_diag_log_store_t *store)
{
    if (store == NULL || !store->initialized) {
        return 0;
    }
    return store->retained_events;
}

void denzic_diag_log_store_dump(denzic_diag_log_store_t *store)
{
    if (store == NULL || !store->initialized) {
        return;
    }

    store_set_dumping(store, true);
    uint16_t start_sec = 0;
    if (!find_oldest_sector(store, &start_sec)) {
        store_set_dumping(store, false);
        store_log(store, DENZIC_DIAG_LOG_STORE_LOG_INFO, "DIAGLOG DUMP: 0 events");
        return;
    }

    denzic_diag_log_event_t *sector_events =
        (denzic_diag_log_event_t *)malloc(DENZIC_DIAG_LOG_EVENTS_PER_SECTOR * sizeof(denzic_diag_log_event_t));
    if (sector_events == NULL) {
        store_set_dumping(store, false);
        store_log(store, DENZIC_DIAG_LOG_STORE_LOG_WARN,
                  "DIAGLOG DUMP: sector snapshot allocation failed");
        return;
    }

    uint32_t dumped = 0;
    for (uint32_t i = 0; i < store->total_sectors; i++) {
        uint16_t sec = (start_sec + (uint16_t)i) % (uint16_t)store->total_sectors;
        uint16_t count = snapshot_sector_events(store, sec, sector_events,
                                                (uint16_t)DENZIC_DIAG_LOG_EVENTS_PER_SECTOR);
        if (count == 0) {
            continue;
        }

        for (uint16_t idx = 0; idx < count; idx++) {
            dump_event_json(store, &sector_events[idx]);
            dumped++;
            pace_dump_output(store, dumped);
        }
    }

    free(sector_events);
    store_set_dumping(store, false);
    store_log(store, DENZIC_DIAG_LOG_STORE_LOG_INFO, "DIAGLOG DUMP: %" PRIu32 " events", dumped);
}

static void store_dump_last_filtered(denzic_diag_log_store_t *store,
                                     uint32_t count,
                                     uint16_t source,
                                     bool use_source_filter)
{
    store_set_dumping(store, true);
    uint16_t newest_sector = 0;
    uint32_t retained = 0;
    store_lock(store);
    retained = store->retained_events;
    if (retained > 0U) {
        newest_sector = store->write_offset > 0U
            ? store->write_sector
            : (uint16_t)((store->write_sector + store->total_sectors - 1U) % store->total_sectors);
    }
    store_unlock(store);
    if (retained == 0U) {
        store_set_dumping(store, false);
        store_log(store, DENZIC_DIAG_LOG_STORE_LOG_INFO,
                  "DIAGLOG LAST %" PRIu32 ": dumped 0 events", count);
        return;
    }

    uint32_t limit = capped_last_count(store, count);
    if (limit == 0) {
        store_set_dumping(store, false);
        store_log(store, DENZIC_DIAG_LOG_STORE_LOG_INFO,
                  "DIAGLOG LAST %" PRIu32 ": dumped 0 events", count);
        return;
    }

    denzic_diag_log_event_t *sector_events =
        (denzic_diag_log_event_t *)malloc(DENZIC_DIAG_LOG_EVENTS_PER_SECTOR * sizeof(denzic_diag_log_event_t));
    denzic_diag_log_event_t *matches =
        (denzic_diag_log_event_t *)malloc(limit * sizeof(denzic_diag_log_event_t));
    if (sector_events == NULL || matches == NULL) {
        free(sector_events);
        free(matches);
        store_set_dumping(store, false);
        store_log(store, DENZIC_DIAG_LOG_STORE_LOG_WARN,
                  "DIAGLOG LAST %" PRIu32 ": snapshot allocation failed", count);
        return;
    }

    uint32_t kept = 0;
    uint32_t scanned = 0;
    uint32_t scan_limit = use_source_filter ? DENZIC_DIAG_LOG_SOURCE_LAST_MAX_SCANNED_EVENTS : limit;
    if (scan_limit < limit) {
        scan_limit = limit;
    }
    scan_limit = scan_limit > retained ? retained : scan_limit;
    for (uint32_t offset = 0; offset < store->total_sectors && scanned < scan_limit && kept < limit; offset++) {
        uint16_t sec = (uint16_t)((newest_sector + store->total_sectors - offset) % store->total_sectors);
        uint16_t sector_count = snapshot_sector_events(store, sec, sector_events,
                                                       (uint16_t)DENZIC_DIAG_LOG_EVENTS_PER_SECTOR);
        if (sector_count == 0) {
            continue;
        }

        for (uint16_t idx = sector_count; idx > 0 && scanned < scan_limit && kept < limit; idx--) {
            denzic_diag_log_event_t *event = &sector_events[idx - 1];
            scanned++;
            if (use_source_filter && event->source != source) {
                continue;
            }
            if (kept < limit) {
                matches[kept++] = *event;
            }
        }
    }

    uint32_t dumped = 0;
    for (uint32_t idx = kept; idx > 0; idx--) {
        dump_event_json(store, &matches[idx - 1]);
        dumped++;
        pace_dump_output(store, dumped);
    }

    free(matches);
    free(sector_events);
    store_set_dumping(store, false);
    if (use_source_filter) {
        const char *name = store_source_name(store, source);
        char source_label[24];
        if (name == NULL) {
            snprintf(source_label, sizeof(source_label), "%u", (unsigned)source);
            name = source_label;
        }
        store_log(
            store,
            DENZIC_DIAG_LOG_STORE_LOG_INFO,
            "DIAGLOG LAST %" PRIu32 " source=%s: dumped %" PRIu32
            " matching events after bounded scan=%" PRIu32 " limit=%" PRIu32,
            count,
            name,
            dumped,
            scanned,
            scan_limit);
    } else {
        store_log(
            store,
            DENZIC_DIAG_LOG_STORE_LOG_INFO,
            "DIAGLOG LAST %" PRIu32 ": dumped %" PRIu32
            " events after scan=%" PRIu32,
            count,
            dumped,
            scanned);
    }
}

void denzic_diag_log_store_dump_last(denzic_diag_log_store_t *store, uint32_t count)
{
    if (store == NULL || !store->initialized) {
        return;
    }
    store_dump_last_filtered(store, count, 0, false);
}

void denzic_diag_log_store_dump_last_by_source(denzic_diag_log_store_t *store,
                                               uint32_t count,
                                               uint16_t source)
{
    if (store == NULL || !store->initialized) {
        return;
    }
    store_dump_last_filtered(store, count, source, true);
}

void denzic_diag_log_store_clear(denzic_diag_log_store_t *store)
{
    if (store == NULL || !store->initialized) {
        return;
    }

    store_lock(store);

    for (uint32_t i = 0; i < store->total_sectors; i++) {
        erase_sector(store, (uint16_t)i);
    }

    store->write_sector = 0;
    store->write_offset = 0;
    store->sector_sequence = 1;
    store->retained_events = 0;
    if (store->sector_counts != NULL) {
        memset(store->sector_counts, 0, store->total_sectors * sizeof(store->sector_counts[0]));
    }
    if (store->sector_sequences != NULL) {
        memset(store->sector_sequences, 0, store->total_sectors * sizeof(store->sector_sequences[0]));
    }

    store_unlock(store);
    store_log(store, DENZIC_DIAG_LOG_STORE_LOG_INFO, "DIAGLOG CLEAR: all logs erased");
}

bool denzic_diag_log_store_is_dumping(denzic_diag_log_store_t *store)
{
    if (store == NULL || !store->initialized) {
        return false;
    }
    store_lock(store);
    bool dumping = store->dumping;
    store_unlock(store);
    return dumping;
}

uint32_t denzic_diag_log_store_read_range(denzic_diag_log_store_t *store,
                                          uint32_t offset,
                                          uint32_t limit,
                                          void *buffer,
                                          uint32_t buffer_size)
{
    if (store == NULL || !store->initialized || buffer == NULL) {
        return 0;
    }

    if (limit == 0 || buffer_size < DENZIC_DIAG_LOG_EVENT_WIRE_BYTES) {
        return 0;
    }

    uint32_t max_events = buffer_size / DENZIC_DIAG_LOG_EVENT_WIRE_BYTES;
    if (limit > max_events) {
        limit = max_events;
    }

    store_read_slot_t *slots = (store_read_slot_t *)calloc(limit, sizeof(store_read_slot_t));
    if (slots == NULL) {
        return 0;
    }

    store_lock(store);

    /* Find minimum sequence to start iteration in chronological order. */
    uint16_t min_seq = UINT16_MAX;
    uint16_t start_sec = 0;
    bool have_start = false;
    if (store->sector_counts != NULL && store->sector_sequences != NULL) {
        for (uint32_t sec = 0; sec < store->total_sectors; sec++) {
            if (store->sector_counts[sec] == 0) {
                continue;
            }
            if (!have_start || store->sector_sequences[sec] < min_seq) {
                min_seq = store->sector_sequences[sec];
                start_sec = (uint16_t)sec;
                have_start = true;
            }
        }
    }

    uint32_t global_idx = 0;
    uint32_t selected = 0;

    for (uint32_t i = 0; have_start && i < store->total_sectors && selected < limit; i++) {
        uint16_t sec = (start_sec + (uint16_t)i) % (uint16_t)store->total_sectors;
        uint16_t sector_count = store->sector_counts[sec];
        if (sector_count == 0) {
            continue;
        }

        for (uint16_t idx = 0; idx < sector_count && selected < limit; idx++, global_idx++) {
            if (global_idx < offset) {
                continue;
            }

            slots[selected].sector = sec;
            slots[selected].index = idx;
            slots[selected].sequence = store->sector_sequences[sec];
            selected++;
        }
    }

    store_unlock(store);

    uint32_t written = 0;
    uint8_t *out = (uint8_t *)buffer;
    for (uint32_t i = 0; i < selected; i++) {
        denzic_diag_log_sector_header_t header;
        if (!read_sector_header(store, slots[i].sector, &header)
            || header.magic != DENZIC_DIAG_LOG_STORE_MAGIC
            || header.sequence != slots[i].sequence) {
            continue;
        }

        denzic_diag_log_event_t event;
        if (!read_event(store, slots[i].sector, slots[i].index, &event)
            || event_is_erased(&event)) {
            continue;
        }

        memcpy(out + written * DENZIC_DIAG_LOG_EVENT_WIRE_BYTES, &event,
               DENZIC_DIAG_LOG_EVENT_WIRE_BYTES);
        written++;
    }

    free(slots);
    return written;
}
