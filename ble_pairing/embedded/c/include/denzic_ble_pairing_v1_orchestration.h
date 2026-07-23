#ifndef DENZIC_BLE_PAIRING_V1_ORCHESTRATION_H
#define DENZIC_BLE_PAIRING_V1_ORCHESTRATION_H

#include <stdbool.h>
#include <stdint.h>

#include "denzic_ble_pairing_v1_generated.h"

/*
 * Portable BLE connection-lifecycle orchestration decisions. This layer sits
 * above the pure policy tables in denzic_ble_pairing_v1.h: it answers "when,
 * in which order, and what follows" for advertising restarts, payload profile
 * selection, bond-delete recovery sequencing, and identity-reset triggers.
 * Every function is a pure decision: sampled state in, a code out. Timers,
 * BLE-stack calls, LED output, storage, and task plumbing stay in the calling
 * product adapter, which samples each input at the point its own sequencing
 * requires. Normative semantics live in
 * ble_pairing/protocol/ble_pairing_v1.md (section 8).
 */

#ifdef __cplusplus
extern "C" {
#endif

/* Advertising restart routing after a disconnect (protocol section 8.1).
 * Priority: shutdown quiesce, then key-wake-only idle, then an active async
 * bond delete (its worker owns the single recovery advertising start);
 * otherwise restart. The caller samples shutdown_quiesce before running its
 * keep-connectable recovery hook and key_wake_only / bond_delete_active
 * after it, matching the event-path ordering. */
uint8_t denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
    bool shutdown_quiesce,
    bool key_wake_only,
    bool bond_delete_active);

/* Advertising restart routing after an advertising-complete event (protocol
 * section 8.1). Same suppress priority, but a bond delete never defers here:
 * the completion event belongs to the worker-owned advertising cycle. */
uint8_t denzic_ble_pairing_v1_orch_adv_restart_after_adv_complete(
    bool shutdown_quiesce,
    bool key_wake_only);

/* Duplicate disconnect-event filter (protocol section 8.2): a repeated event
 * for the same connection inside the filter window is a stack echo. The
 * caller compares connection handle and reason itself. */
bool denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(
    int64_t previous_event_at_ms,
    int64_t now_ms);

/* Advertising payload profile plan (protocol section 8.3). A Type-controlled
 * recovery payload is built first, but only when no Swift Pair prompt
 * competes; Swift Pair is tried next; anything unresolved falls back to the
 * normal payload. Payload construction stays in the adapter and may fail. */
bool denzic_ble_pairing_v1_orch_adv_profile_try_type_recovery(
    bool type_recovery_requested,
    bool swift_pair_requested);

bool denzic_ble_pairing_v1_orch_adv_profile_try_swift_pair(
    bool type_recovery_enabled,
    bool swift_pair_requested);

uint8_t denzic_ble_pairing_v1_orch_adv_profile_select(
    bool type_recovery_enabled,
    bool swift_pair_enabled);

/* Local IRK reset trigger after a recovery disconnect (protocol section 8.4):
 * only inside an open pairing window, only when an identity rotation is
 * pending, and only once a successful bond lookup proves no bonds remain. */
bool denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
    bool pairing_window_open,
    bool identity_rotate_pending,
    bool bond_lookup_ok,
    int bonded_peer_count);

/* Bond-delete recovery sequencing decisions (protocol section 8.5). The
 * adapter owns the actual step order and every side effect; these predicates
 * pin which branch each step takes. */

/* A bounded non-connectable warm-up advertisement runs before the peer
 * cleanup only for a Type-controlled recovery with a known current peer. */
bool denzic_ble_pairing_v1_orch_bond_delete_warmup_before_delete(
    bool type_controlled,
    bool known_peer);

/* Direct single-peer delete without enumeration applies only for a
 * Type-controlled recovery with a known current peer; anything else falls
 * back to enumerating bonded peers. */
bool denzic_ble_pairing_v1_orch_bond_delete_direct_peer_delete(
    bool type_controlled,
    bool known_peer);

/* Per-peer removal during enumeration: a native recovery must run the GAP
 * unpair so the local IRK rotates once the final bond disappears; a
 * Type-controlled recovery deletes the peer records but keeps the local
 * identity for the host's re-pair. */
bool denzic_ble_pairing_v1_orch_bond_delete_use_unpair_api(bool type_controlled);

/* Cleanup success: a direct delete skips the enumeration outcome; an
 * enumeration must have succeeded and no per-peer delete may have failed. */
bool denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(
    bool fell_back_to_enumeration,
    int lookup_rc,
    int first_delete_rc);

/* After the recovery advertising start, the pairing window guard is
 * re-activated only when it lapsed while the window is still open. */
bool denzic_ble_pairing_v1_orch_bond_delete_reactivate_window(
    bool power_blocker_active,
    bool pairing_window_active);

/* Passive reattach evidence classification (protocol section 8.6): the host
 * may reclaim the link when Windows already shows a pairing with a live
 * native HID endpoint, or when a fresh native HID address appeared after the
 * monitoring baseline. Address-set diffing stays in the adapter. */
bool denzic_ble_pairing_v1_orch_reattach_evidence_ready(
    bool paired_devices_visible,
    bool native_hid_present,
    bool fresh_native_hid_after_baseline);

#ifdef __cplusplus
}
#endif

#endif
