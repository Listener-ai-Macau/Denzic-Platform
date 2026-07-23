#include "denzic_ble_pairing_v1_orchestration.h"

#include <stdio.h>

static int failures = 0;

#define CHECK(condition)                                                     \
    do {                                                                     \
        if (!(condition)) {                                                  \
            failures++;                                                      \
            printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition);      \
        }                                                                    \
    } while (0)

static void test_adv_restart_after_disconnect_priority(void)
{
    /* Every suppressor wins over a restart, in fixed priority order. */
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
              false, false, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_RESTART);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
              true, false, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_SHUTDOWN);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
              false, true, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_KEY_WAKE);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
              false, false, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_DEFER_BOND_DELETE);
    /* Shutdown outranks key-wake; key-wake outranks the bond-delete defer. */
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
              true, true, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_SHUTDOWN);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
              false, true, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_KEY_WAKE);
}

static void test_adv_restart_after_adv_complete_never_defers(void)
{
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_adv_complete(
              false, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_RESTART);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_adv_complete(
              true, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_SHUTDOWN);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_adv_complete(
              false, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_KEY_WAKE);
    CHECK(denzic_ble_pairing_v1_orch_adv_restart_after_adv_complete(
              true, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_SHUTDOWN);
}

static void test_disconnect_duplicate_filter_window(void)
{
    /* Inside the filter window the repeated event is a stack echo. */
    CHECK(denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(1000, 1000));
    CHECK(denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(1000, 1749));
    /* The window edge and anything beyond it is a real new disconnect. */
    CHECK(!denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(1000, 1750));
    CHECK(!denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(1000, 5000));
    /* A clock moving backwards is never a duplicate. */
    CHECK(!denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(1000, 999));
    /* The window length comes from the protocol contract. */
    CHECK(DENZIC_BLE_PAIRING_V1_DISCONNECT_DUPLICATE_FILTER_MS == 750u);
}

static void test_adv_profile_plan(void)
{
    /* Type recovery is only attempted when Swift Pair does not compete. */
    CHECK(denzic_ble_pairing_v1_orch_adv_profile_try_type_recovery(true, false));
    CHECK(!denzic_ble_pairing_v1_orch_adv_profile_try_type_recovery(true, true));
    CHECK(!denzic_ble_pairing_v1_orch_adv_profile_try_type_recovery(false, false));

    /* Swift Pair loses to an already-enabled Type recovery payload. */
    CHECK(denzic_ble_pairing_v1_orch_adv_profile_try_swift_pair(false, true));
    CHECK(!denzic_ble_pairing_v1_orch_adv_profile_try_swift_pair(true, true));
    CHECK(!denzic_ble_pairing_v1_orch_adv_profile_try_swift_pair(false, false));

    /* Final selection: Swift Pair first, then Type recovery, then normal. */
    CHECK(denzic_ble_pairing_v1_orch_adv_profile_select(false, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_PROFILE_SWIFT_PAIR);
    CHECK(denzic_ble_pairing_v1_orch_adv_profile_select(true, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_PROFILE_TYPE_RECOVERY);
    CHECK(denzic_ble_pairing_v1_orch_adv_profile_select(true, true) ==
          DENZIC_BLE_PAIRING_V1_ADV_PROFILE_SWIFT_PAIR);
    CHECK(denzic_ble_pairing_v1_orch_adv_profile_select(false, false) ==
          DENZIC_BLE_PAIRING_V1_ADV_PROFILE_NORMAL);
}

static void test_irk_reset_after_disconnect_gate(void)
{
    CHECK(denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
              true, true, true, 0));
    /* A surviving bond keeps the current IRK. */
    CHECK(!denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
              true, true, true, 1));
    /* A failed lookup must not reset the IRK on an unknown bond state. */
    CHECK(!denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
              true, true, false, 0));
    CHECK(!denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
              false, true, true, 0));
    CHECK(!denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
              true, false, true, 0));
}

static void test_bond_delete_sequencing(void)
{
    /* Warm-up and direct delete are Type-controlled-with-known-peer only. */
    CHECK(denzic_ble_pairing_v1_orch_bond_delete_warmup_before_delete(true, true));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_warmup_before_delete(true, false));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_warmup_before_delete(false, true));
    CHECK(denzic_ble_pairing_v1_orch_bond_delete_direct_peer_delete(true, true));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_direct_peer_delete(false, true));

    /* Native recovery unpairs through GAP; Type recovery deletes records. */
    CHECK(denzic_ble_pairing_v1_orch_bond_delete_use_unpair_api(false));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_use_unpair_api(true));

    /* Direct delete skips the enumeration outcome entirely. */
    CHECK(denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(false, -1, 0));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(false, 0, -5));
    CHECK(denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(true, 0, 0));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(true, -1, 0));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(true, 0, -2));

    /* Window re-activation only when the guard lapsed and the window lives. */
    CHECK(denzic_ble_pairing_v1_orch_bond_delete_reactivate_window(false, true));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_reactivate_window(true, true));
    CHECK(!denzic_ble_pairing_v1_orch_bond_delete_reactivate_window(false, false));
}

static void test_reattach_evidence_classification(void)
{
    /* An existing Windows pairing only counts with a live HID endpoint. */
    CHECK(denzic_ble_pairing_v1_orch_reattach_evidence_ready(true, true, false));
    CHECK(!denzic_ble_pairing_v1_orch_reattach_evidence_ready(true, false, false));
    /* A fresh HID address after the baseline is evidence on its own. */
    CHECK(denzic_ble_pairing_v1_orch_reattach_evidence_ready(false, false, true));
    CHECK(denzic_ble_pairing_v1_orch_reattach_evidence_ready(false, true, true));
    CHECK(!denzic_ble_pairing_v1_orch_reattach_evidence_ready(false, false, false));
}

int main(void)
{
    test_adv_restart_after_disconnect_priority();
    test_adv_restart_after_adv_complete_never_defers();
    test_disconnect_duplicate_filter_window();
    test_adv_profile_plan();
    test_irk_reset_after_disconnect_gate();
    test_bond_delete_sequencing();
    test_reattach_evidence_classification();
    if (failures != 0) {
        printf("%d orchestration test failure(s)\n", failures);
        return 1;
    }
    return 0;
}
