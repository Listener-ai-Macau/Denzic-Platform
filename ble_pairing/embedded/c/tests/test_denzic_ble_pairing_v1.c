#include "denzic_ble_pairing_v1.h"

#include <stdio.h>

static int failures = 0;

#define CHECK(condition)                                                     \
    do {                                                                     \
        if (!(condition)) {                                                  \
            failures++;                                                      \
            printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition);      \
        }                                                                    \
    } while (0)

static void test_disconnect_classification(void)
{
    /* Manual host delete (HCI 0x13 composed with the stack base) must be
     * recognised so the device never chases that host. */
    CHECK(denzic_ble_pairing_v1_classify_disconnect(531, false) ==
          DENZIC_BLE_PAIRING_V1_DISCONNECT_HOST_DELIBERATE_UNPAIR);
    CHECK(denzic_ble_pairing_v1_classify_disconnect(
              (int32_t)DENZIC_BLE_PAIRING_V1_HCI_NIMBLE_HCI_STATUS_BASE +
                  (int32_t)DENZIC_BLE_PAIRING_V1_HCI_REMOTE_USER_TERMINATED_REASON,
              false) ==
          DENZIC_BLE_PAIRING_V1_DISCONNECT_HOST_DELIBERATE_UNPAIR);
    /* Any other status is transient link loss. */
    CHECK(denzic_ble_pairing_v1_classify_disconnect(0x08, false) ==
          DENZIC_BLE_PAIRING_V1_DISCONNECT_TRANSIENT_LINK_LOSS);
    CHECK(denzic_ble_pairing_v1_classify_disconnect(530, false) ==
          DENZIC_BLE_PAIRING_V1_DISCONNECT_TRANSIENT_LINK_LOSS);
    CHECK(denzic_ble_pairing_v1_classify_disconnect(532, false) ==
          DENZIC_BLE_PAIRING_V1_DISCONNECT_TRANSIENT_LINK_LOSS);
    /* Caller context overrides: a locally requested terminate is never
     * mistaken for a host delete even when it carries the same reason. */
    CHECK(denzic_ble_pairing_v1_classify_disconnect(531, true) ==
          DENZIC_BLE_PAIRING_V1_DISCONNECT_LOCAL_REQUEST);
}

static void test_advertising_after_disconnect(void)
{
    /* No-chase rule: deliberate unpair never earns directed reconnect. */
    CHECK(denzic_ble_pairing_v1_advertising_after_disconnect(
              DENZIC_BLE_PAIRING_V1_DISCONNECT_HOST_DELIBERATE_UNPAIR, false) ==
          DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_UNDIRECTED_ONLY);
    /* Transient loss keeps the directed reconnect path. */
    CHECK(denzic_ble_pairing_v1_advertising_after_disconnect(
              DENZIC_BLE_PAIRING_V1_DISCONNECT_TRANSIENT_LINK_LOSS, false) ==
          DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_DIRECTED_RECONNECT);
    /* Bond cleanup owns the single recovery advertising start. */
    CHECK(denzic_ble_pairing_v1_advertising_after_disconnect(
              DENZIC_BLE_PAIRING_V1_DISCONNECT_TRANSIENT_LINK_LOSS, true) ==
          DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_SUPPRESS);
    CHECK(denzic_ble_pairing_v1_advertising_after_disconnect(
              DENZIC_BLE_PAIRING_V1_DISCONNECT_HOST_DELIBERATE_UNPAIR, true) ==
          DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_SUPPRESS);
}

static void test_directed_reconnect_gate(void)
{
    CHECK(denzic_ble_pairing_v1_directed_reconnect_allowed(true, false, false));
    CHECK(!denzic_ble_pairing_v1_directed_reconnect_allowed(false, false, false));
    CHECK(!denzic_ble_pairing_v1_directed_reconnect_allowed(true, true, false));
    /* An open pairing window always advertises undirected. */
    CHECK(!denzic_ble_pairing_v1_directed_reconnect_allowed(true, false, true));
}

static void test_bond_delete_suppression(void)
{
    CHECK(!denzic_ble_pairing_v1_advertising_allowed_during_bond_delete(false, false));
    CHECK(!denzic_ble_pairing_v1_advertising_allowed_during_bond_delete(false, true));
    CHECK(!denzic_ble_pairing_v1_advertising_allowed_during_bond_delete(true, false));
    /* Bounded non-connectable warm-up is the only permitted exception. */
    CHECK(denzic_ble_pairing_v1_advertising_allowed_during_bond_delete(true, true));
}

static void test_window_remaining(void)
{
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(0, 120000, 500) == 0);
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(100, 0, 500) == 0);
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(-5, 120000, 500) == 0);
    /* Clock running backwards keeps the full window. */
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(1000, 120000, 500) == 120000);
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(1000, 120000, 120999) == 1);
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(1000, 120000, 121000) == 0);
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(1000, 120000, 200000) == 0);
    CHECK(denzic_ble_pairing_v1_window_remaining_ms(1000, 120000, 61000) == 60000);
}

static void test_swift_pair_adv_duration(void)
{
    CHECK(denzic_ble_pairing_v1_swift_pair_adv_duration_ms(0) == 1000);
    CHECK(denzic_ble_pairing_v1_swift_pair_adv_duration_ms(999) == 1000);
    CHECK(denzic_ble_pairing_v1_swift_pair_adv_duration_ms(1000) == 1000);
    CHECK(denzic_ble_pairing_v1_swift_pair_adv_duration_ms(45000) == 45000);
    CHECK(denzic_ble_pairing_v1_swift_pair_adv_duration_ms(0x80000000LL) == 0x7fffffff);
}

static void test_swift_pair_prompt_evaluation(void)
{
    CHECK(denzic_ble_pairing_v1_evaluate_swift_pair_prompt(false, false, 45000, 30000) ==
          DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_ACTIVE);
    /* Silent Type-controlled recovery suppresses (and consumes) the prompt. */
    CHECK(denzic_ble_pairing_v1_evaluate_swift_pair_prompt(true, false, 45000, 30000) ==
          DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_SUPPRESSED);
    CHECK(denzic_ble_pairing_v1_evaluate_swift_pair_prompt(false, true, 45000, 30000) ==
          DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_CONSUMED);
    CHECK(denzic_ble_pairing_v1_evaluate_swift_pair_prompt(false, false, 0, 30000) ==
          DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_DISABLED);
    CHECK(denzic_ble_pairing_v1_evaluate_swift_pair_prompt(false, false, 45000, 0) ==
          DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_EXPIRED);
}

static void test_window_close_on_secure(void)
{
    /* Type-controlled recovery waits for the Type audio-ready signal. */
    CHECK(denzic_ble_pairing_v1_window_close_on_secure(true, true) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN);
    CHECK(denzic_ble_pairing_v1_window_close_on_secure(true, false) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW);
    CHECK(denzic_ble_pairing_v1_window_close_on_secure(false, true) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW);
    CHECK(denzic_ble_pairing_v1_window_close_on_secure(false, false) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW);
}

static void test_window_close_on_type_audio_ready(void)
{
    /* TYPE-ready close rule: secure link inside an open window closes it. */
    CHECK(denzic_ble_pairing_v1_window_close_on_type_audio_ready(true, false, true, true, false) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW);
    CHECK(denzic_ble_pairing_v1_window_close_on_type_audio_ready(true, false, true, false, true) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW);
    /* Audio ready on an insecure link keeps the window open. */
    CHECK(denzic_ble_pairing_v1_window_close_on_type_audio_ready(true, false, true, false, false) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN);
    /* Pre-reset connection still draining: ignore the signal. */
    CHECK(denzic_ble_pairing_v1_window_close_on_type_audio_ready(true, true, true, true, true) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN);
    /* No window, or no valid descriptor: nothing to close. */
    CHECK(denzic_ble_pairing_v1_window_close_on_type_audio_ready(false, false, true, true, true) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN);
    CHECK(denzic_ble_pairing_v1_window_close_on_type_audio_ready(true, false, false, false, false) ==
          DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN);
}

static void test_type_controlled_recovery(void)
{
    CHECK(denzic_ble_pairing_v1_type_controlled_recovery(true, false, false, false));
    CHECK(denzic_ble_pairing_v1_type_controlled_recovery(false, true, false, false));
    /* A recent host marker counts only while the link is live. */
    CHECK(denzic_ble_pairing_v1_type_controlled_recovery(false, false, true, true));
    CHECK(!denzic_ble_pairing_v1_type_controlled_recovery(false, false, true, false));
    CHECK(!denzic_ble_pairing_v1_type_controlled_recovery(false, false, false, true));
}

static void test_identity_for_recovery(void)
{
    /* Type-controlled recovery keeps the stable identity (no IRK rotation). */
    CHECK(denzic_ble_pairing_v1_identity_for_recovery(true, true) ==
          DENZIC_BLE_PAIRING_V1_IDENTITY_KEEP_STABLE);
    CHECK(denzic_ble_pairing_v1_identity_for_recovery(true, false) ==
          DENZIC_BLE_PAIRING_V1_IDENTITY_KEEP_STABLE);
    /* Native recovery on a live link defers rotation until disconnect. */
    CHECK(denzic_ble_pairing_v1_identity_for_recovery(false, true) ==
          DENZIC_BLE_PAIRING_V1_IDENTITY_DEFER_ROTATE_UNTIL_DISCONNECT);
    /* Idle native recovery rotates immediately. */
    CHECK(denzic_ble_pairing_v1_identity_for_recovery(false, false) ==
          DENZIC_BLE_PAIRING_V1_IDENTITY_ROTATE_FRESH);
}

static void test_rotate_before_advertising(void)
{
    /* Delete-then-rotate-then-advertise ordering: a pending deferred
     * rotation must run before the recovery advertising start. */
    CHECK(denzic_ble_pairing_v1_rotate_before_advertising(true));
    CHECK(!denzic_ble_pairing_v1_rotate_before_advertising(false));
}

static void test_refresh_existing_window(void)
{
    CHECK(denzic_ble_pairing_v1_refresh_existing_pairing_window(true, 0, false, false));
    CHECK(!denzic_ble_pairing_v1_refresh_existing_pairing_window(false, 0, false, false));
    CHECK(!denzic_ble_pairing_v1_refresh_existing_pairing_window(true, 1, false, false));
    CHECK(!denzic_ble_pairing_v1_refresh_existing_pairing_window(true, 0, true, false));
    CHECK(!denzic_ble_pairing_v1_refresh_existing_pairing_window(true, 0, false, true));
}

static void test_security_failure_action(void)
{
    /* Encryption failure inside the window keeps pairing advertising
     * available for the host retry instead of restarting repair. */
    CHECK(denzic_ble_pairing_v1_security_failure_action(true) ==
          DENZIC_BLE_PAIRING_V1_SECURITY_FAILURE_RETRY_WITHIN_WINDOW);
    /* Outside the window a security failure opens a full repair. */
    CHECK(denzic_ble_pairing_v1_security_failure_action(false) ==
          DENZIC_BLE_PAIRING_V1_SECURITY_FAILURE_OPEN_REPAIR_WINDOW);
}

int main(void)
{
    test_disconnect_classification();
    test_advertising_after_disconnect();
    test_directed_reconnect_gate();
    test_bond_delete_suppression();
    test_window_remaining();
    test_swift_pair_adv_duration();
    test_swift_pair_prompt_evaluation();
    test_window_close_on_secure();
    test_window_close_on_type_audio_ready();
    test_type_controlled_recovery();
    test_identity_for_recovery();
    test_rotate_before_advertising();
    test_refresh_existing_window();
    test_security_failure_action();

    if (failures != 0) {
        printf("%d check(s) failed\n", failures);
        return 1;
    }
    printf("all denzic_ble_pairing_v1 decision checks passed\n");
    return 0;
}
