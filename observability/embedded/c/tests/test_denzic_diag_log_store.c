#include "denzic_diag_log_store.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

#define TEST_SECTOR_SIZE 4096u
#define TEST_SECTORS 4u

#define TEST_MAX_LINES 256u
#define TEST_LINE_BYTES 192u

static uint8_t s_storage[TEST_SECTORS * TEST_SECTOR_SIZE];
static uint32_t s_now_ms;
static unsigned s_lock_depth;
static unsigned s_lock_imbalance;
static char s_lines[TEST_MAX_LINES][TEST_LINE_BYTES];
static unsigned s_line_count;

static bool test_storage_read(void *context, uint32_t offset, uint8_t *buffer, size_t length)
{
    (void)context;
    if ((size_t)offset + length > sizeof(s_storage)) {
        return false;
    }
    memcpy(buffer, s_storage + offset, length);
    return true;
}

static bool test_storage_write(void *context, uint32_t offset, const uint8_t *data, size_t length)
{
    (void)context;
    if ((size_t)offset + length > sizeof(s_storage)) {
        return false;
    }
    memcpy(s_storage + offset, data, length);
    return true;
}

static bool test_storage_erase(void *context, uint32_t offset, size_t length)
{
    (void)context;
    if ((size_t)offset + length > sizeof(s_storage)) {
        return false;
    }
    memset(s_storage + offset, 0xFF, length);
    return true;
}

static uint32_t test_timestamp_ms(void *context)
{
    (void)context;
    return s_now_ms;
}

static void test_lock(void *context)
{
    (void)context;
    s_lock_depth++;
}

static void test_unlock(void *context)
{
    (void)context;
    if (s_lock_depth == 0) {
        s_lock_imbalance++;
        return;
    }
    s_lock_depth--;
}

static void test_emit_line(void *context, const char *line)
{
    (void)context;
    assert(s_line_count < TEST_MAX_LINES);
    snprintf(s_lines[s_line_count], TEST_LINE_BYTES, "%s", line);
    s_line_count++;
}

static denzic_diag_log_store_port_t test_port(void)
{
    denzic_diag_log_store_port_t port = {
        .storage_read = test_storage_read,
        .storage_write = test_storage_write,
        .storage_erase = test_storage_erase,
        .timestamp_ms = test_timestamp_ms,
        .lock = test_lock,
        .unlock = test_unlock,
        .log = NULL,
        .emit_line = test_emit_line,
        .pace = NULL,
        .source_name = NULL,
    };
    return port;
}

static void init_test_store(denzic_diag_log_store_t *store, denzic_diag_log_store_port_t *port)
{
    bool ok = denzic_diag_log_store_init(store, port, NULL, sizeof(s_storage));
    assert(ok);
    (void)ok;
}

static void reset_storage(void)
{
    memset(s_storage, 0xFF, sizeof(s_storage));
    s_now_ms = 0;
    s_line_count = 0;
}

static void test_write_read_and_range(void)
{
    reset_storage();
    denzic_diag_log_store_t store;
    denzic_diag_log_store_port_t port = test_port();
    init_test_store(&store, &port);
    assert(denzic_diag_log_store_count(&store) == 0);
    assert(!denzic_diag_log_store_is_dumping(&store));

    s_now_ms = 1000;
    for (uint32_t i = 0; i < 3; i++) {
        denzic_diag_log_store_write(&store, 0x02u, (uint8_t)(0x10u + i), (uint8_t)i,
                                    100u + i, 200u + i, 300u + i, 400u + i);
        s_now_ms += 10;
    }
    assert(denzic_diag_log_store_count(&store) == 3);

    denzic_diag_log_event_t out[4];
    uint32_t n = denzic_diag_log_store_read_range(&store, 0, 10, out, sizeof(out));
    assert(n == 3);
    for (uint32_t i = 0; i < 3; i++) {
        assert(out[i].timestamp_ms == 1000u + 10u * i);
        assert(out[i].source == 0x02u);
        assert(out[i].event == (uint8_t)(0x10u + i));
        assert(out[i].severity == (uint8_t)i);
        assert(out[i].arg1 == 100u + i);
        assert(out[i].arg2 == 200u + i);
        assert(out[i].arg3 == 300u + i);
        assert(out[i].arg4 == 400u + i);
    }

    n = denzic_diag_log_store_read_range(&store, 1, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 101u);

    /* Buffer smaller than one event yields nothing. */
    n = denzic_diag_log_store_read_range(&store, 0, 10, out, 8);
    assert(n == 0);

    /* Full dump emits one line per retained event, oldest first. */
    s_line_count = 0;
    denzic_diag_log_store_dump(&store);
    assert(s_line_count == 3);
    assert(strstr(s_lines[0], "\"a1\":100") != NULL);
    assert(strstr(s_lines[2], "\"a1\":102") != NULL);
    assert(!denzic_diag_log_store_is_dumping(&store));

    denzic_diag_log_store_deinit(&store);
    assert(s_lock_depth == 0 && s_lock_imbalance == 0);
}

static void test_dump_last_and_by_source(void)
{
    reset_storage();
    denzic_diag_log_store_t store;
    denzic_diag_log_store_port_t port = test_port();
    init_test_store(&store, &port);

    for (uint32_t i = 0; i < 3; i++) {
        denzic_diag_log_store_write(&store, 0x02u, 0x10u, 0u, 100u + i, 0, 0, 0);
    }
    denzic_diag_log_store_write(&store, 0x05u, 0x20u, 1u, 200u, 0, 0, 0);

    s_line_count = 0;
    denzic_diag_log_store_dump_last(&store, 2);
    assert(s_line_count == 2);
    assert(strstr(s_lines[0], "\"a1\":102") != NULL);
    assert(strstr(s_lines[1], "\"a1\":200") != NULL);

    s_line_count = 0;
    denzic_diag_log_store_dump_last_by_source(&store, 10, 0x02u);
    assert(s_line_count == 3);
    for (unsigned i = 0; i < s_line_count; i++) {
        assert(strstr(s_lines[i], "\"src\":2,") != NULL);
    }
    assert(strstr(s_lines[0], "\"a1\":100") != NULL);
    assert(strstr(s_lines[2], "\"a1\":102") != NULL);

    s_line_count = 0;
    denzic_diag_log_store_dump_last_by_source(&store, 10, 0x05u);
    assert(s_line_count == 1);
    assert(strstr(s_lines[0], "\"a1\":200") != NULL);
    assert(!denzic_diag_log_store_is_dumping(&store));

    denzic_diag_log_store_deinit(&store);
    assert(s_lock_depth == 0 && s_lock_imbalance == 0);
}

static void test_clear_and_persistence(void)
{
    reset_storage();
    denzic_diag_log_store_port_t port = test_port();

    denzic_diag_log_store_t store;
    init_test_store(&store, &port);
    for (uint32_t i = 0; i < 5; i++) {
        denzic_diag_log_store_write(&store, 0x01u, 0x30u, 0u, 500u + i, 0, 0, 0);
    }
    assert(denzic_diag_log_store_count(&store) == 5);
    denzic_diag_log_store_deinit(&store);

    /* Re-init on the same storage retains events across reboots. */
    denzic_diag_log_store_t reopened;
    init_test_store(&reopened, &port);
    assert(denzic_diag_log_store_count(&reopened) == 5);
    denzic_diag_log_event_t out[5];
    uint32_t n = denzic_diag_log_store_read_range(&reopened, 0, 5, out, sizeof(out));
    assert(n == 5);
    for (uint32_t i = 0; i < 5; i++) {
        assert(out[i].arg1 == 500u + i);
    }

    /* New writes after reopen append without corrupting retained events. */
    denzic_diag_log_store_write(&reopened, 0x01u, 0x31u, 0u, 999u, 0, 0, 0);
    assert(denzic_diag_log_store_count(&reopened) == 6);
    n = denzic_diag_log_store_read_range(&reopened, 5, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 999u);

    denzic_diag_log_store_clear(&reopened);
    assert(denzic_diag_log_store_count(&reopened) == 0);
    s_line_count = 0;
    denzic_diag_log_store_dump(&reopened);
    assert(s_line_count == 0);

    denzic_diag_log_store_write(&reopened, 0x01u, 0x32u, 0u, 42u, 0, 0, 0);
    assert(denzic_diag_log_store_count(&reopened) == 1);
    n = denzic_diag_log_store_read_range(&reopened, 0, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 42u);

    denzic_diag_log_store_deinit(&reopened);
    assert(s_lock_depth == 0 && s_lock_imbalance == 0);
}

static void test_wrap_around(void)
{
    reset_storage();
    denzic_diag_log_store_t store;
    denzic_diag_log_store_port_t port = test_port();
    init_test_store(&store, &port);

    /*
     * 4 seed events plus 700 wrap events: 704 written into 4 sectors of 170
     * events each. Wrapping past sector 0 erases its 170 oldest events, so
     * 704 - 170 = 534 events stay retained (events 170..703).
     */
    enum { WRAP_RETAINED = 534 };
    for (uint32_t i = 0; i < 4; i++) {
        denzic_diag_log_store_write(&store, 0x01u, 0x40u, 0u, 100u + i, 0, 0, 0);
    }
    for (uint32_t i = 0; i < 700; i++) {
        denzic_diag_log_store_write(&store, 0x01u, 0x41u, 0u, 1000u + i, 0, 0, 0);
    }
    assert(denzic_diag_log_store_count(&store) == WRAP_RETAINED);

    /* Oldest retained event is write index 170: arg1 = 1000 + (170 - 4). */
    denzic_diag_log_event_t out[1];
    uint32_t n = denzic_diag_log_store_read_range(&store, 0, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 1166u);
    n = denzic_diag_log_store_read_range(&store, WRAP_RETAINED - 1u, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 1699u);

    /* Chronological order is contiguous across the wrap boundary. */
    denzic_diag_log_event_t window[8];
    n = denzic_diag_log_store_read_range(&store, 100, 8, window, sizeof(window));
    assert(n == 8);
    for (uint32_t i = 1; i < 8; i++) {
        assert(window[i].arg1 == window[i - 1].arg1 + 1u);
    }

    /* State survives a reopen after wrap-around. */
    denzic_diag_log_store_deinit(&store);
    denzic_diag_log_store_t reopened;
    init_test_store(&reopened, &port);
    assert(denzic_diag_log_store_count(&reopened) == WRAP_RETAINED);
    n = denzic_diag_log_store_read_range(&reopened, 0, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 1166u);
    denzic_diag_log_store_write(&reopened, 0x01u, 0x42u, 0u, 2000u, 0, 0, 0);
    n = denzic_diag_log_store_read_range(&reopened, WRAP_RETAINED, 1, out, sizeof(out));
    assert(n == 1 && out[0].arg1 == 2000u);

    denzic_diag_log_store_deinit(&reopened);
    assert(s_lock_depth == 0 && s_lock_imbalance == 0);
}

int main(void)
{
    test_write_read_and_range();
    test_dump_last_and_by_source();
    test_clear_and_persistence();
    test_wrap_around();
    return 0;
}
