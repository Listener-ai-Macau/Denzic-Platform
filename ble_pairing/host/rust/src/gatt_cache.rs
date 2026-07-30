//! Host-side GATT cache policy decision table (protocol section 9).
//!
//! Pure scenario-to-policy mapping for cached-vs-uncached GATT access on a
//! host (central) stack. The table decides which cache posture each session
//! scenario uses and in which order cached/uncached attempts run. Mapping a
//! [`GattCacheMode`] to an OS cache-mode API and every side effect (service
//! discovery, session open, retries, logging) stays in the calling product
//! adapter. A peripheral/embedded stack has no GATT client cache concept, so
//! only the protocol constants are mirrored to the C end.

use crate::{
    GATT_CACHE_CACHED_FIRST, GATT_CACHE_CACHED_ONLY, GATT_CACHE_UNCACHED_FIRST,
    GATT_CACHE_UNCACHED_ONLY,
};

/// Cache posture of a single GATT access attempt (protocol section 9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GattCacheMode {
    /// Serve from the host stack's system cache when available.
    Cached,
    /// Bypass the cache; require fresh discovery from the peer.
    Uncached,
}

/// Ordered cache policy for one GATT session scenario (protocol section 9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GattCachePolicy {
    /// Only the cached attempt runs; no fresh discovery is triggered.
    CachedOnly,
    /// Only a fresh (uncached) attempt runs; stale cached handles are never used.
    UncachedOnly,
    /// Fresh discovery first, cached handles only as fallback.
    UncachedFirst,
    /// Cached handles first, fresh discovery as fallback.
    CachedFirst,
}

impl GattCachePolicy {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::CachedOnly => GATT_CACHE_CACHED_ONLY,
            Self::UncachedOnly => GATT_CACHE_UNCACHED_ONLY,
            Self::UncachedFirst => GATT_CACHE_UNCACHED_FIRST,
            Self::CachedFirst => GATT_CACHE_CACHED_FIRST,
        }
    }

    /// Attempt sequence: the modes in the order the adapter must try them.
    pub const fn modes(self) -> &'static [GattCacheMode] {
        match self {
            Self::CachedOnly => &[GattCacheMode::Cached],
            Self::UncachedOnly => &[GattCacheMode::Uncached],
            Self::UncachedFirst => &[GattCacheMode::Uncached, GattCacheMode::Cached],
            Self::CachedFirst => &[GattCacheMode::Cached, GattCacheMode::Uncached],
        }
    }
}

// Scenario table (protocol section 9.2). Each constant names the session
// scenario; the adapter picks the constant for the scenario it is in and maps
// `policy.modes()` onto its OS cache-mode API.

/// Notify-target open while a recently completed pairing may have left a
/// valid system GATT cache.
pub const RECENT_PAIRING_NOTIFY_CACHE_POLICY: GattCachePolicy = GattCachePolicy::CachedFirst;

/// Notify-target open over the known-address list (default discovery posture).
pub const KNOWN_ADDRESS_NOTIFY_CACHE_POLICY: GattCachePolicy = GattCachePolicy::UncachedFirst;

/// Notify-target open by one exact address.
pub const DEVICE_NOTIFY_CACHE_POLICY: GattCachePolicy = GattCachePolicy::UncachedFirst;

/// Notify-target open on a persisted OS-level bond whose system GATT cache
/// can be valid while the stack is temporarily unable to complete a fresh
/// uncached query (for example during link rehydration).
pub const PERSISTED_BOND_NOTIFY_CACHE_POLICY: GattCachePolicy = GattCachePolicy::CachedFirst;

/// Notify-target reopen after a confirmed image handoff. The schema is stable,
/// but the pre-reboot characteristic handles are not; discover the live peer
/// first and retain cached discovery only as a fallback.
pub const POST_CONFIRM_NOTIFY_CACHE_POLICY: GattCachePolicy = GattCachePolicy::UncachedFirst;

/// Fast reachability probe of an already-known service.
pub const SERVICE_REACHABILITY_PROBE_CACHE_POLICY: GattCachePolicy = GattCachePolicy::CachedFirst;

/// Status-target open for a bounded status/capability probe.
pub const STATUS_PROBE_CACHE_POLICY: GattCachePolicy = GattCachePolicy::CachedFirst;

/// Control-characteristic write path: fresh handles are required, cached
/// handles are only a fallback.
pub const CONTROL_WRITE_CACHE_POLICY: GattCachePolicy = GattCachePolicy::UncachedFirst;

/// Diagnostic export path: always fresh discovery.
pub const DIAGNOSTIC_CACHE_POLICY: GattCachePolicy = GattCachePolicy::UncachedOnly;

/// Deadline-bounded service-endpoint open: the bounded variant rehydrates
/// the cache first to fit its time budget.
pub const DEADLINE_SERVICE_ENDPOINT_CACHE_POLICY: GattCachePolicy = GattCachePolicy::CachedFirst;

/// OTA control-target open on a device handle (protocol section 9.3).
///
/// A verified active handoff makes the device handle and access grant
/// reusable, but control writes still require fresh GATT characteristic
/// handles, so no cached fallback is allowed. Without a verified handoff the
/// cached handle remains a fallback after fresh discovery.
pub const fn ota_device_control_cache_policy(verified_active_handoff: bool) -> GattCachePolicy {
    if verified_active_handoff {
        GattCachePolicy::UncachedOnly
    } else {
        GattCachePolicy::UncachedFirst
    }
}

/// OTA service-endpoint open (protocol section 9.3).
///
/// A cached handle may be used only when the caller has proven the endpoint
/// identity is current (`allow_cached`); the uncached attempt always runs
/// first. Without that proof only fresh discovery is allowed.
pub const fn ota_service_endpoint_cache_policy(allow_cached: bool) -> GattCachePolicy {
    if allow_cached {
        GattCachePolicy::UncachedFirst
    } else {
        GattCachePolicy::UncachedOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_codes_match_protocol_constants() {
        assert_eq!(GattCachePolicy::CachedOnly.as_u8(), GATT_CACHE_CACHED_ONLY);
        assert_eq!(
            GattCachePolicy::UncachedOnly.as_u8(),
            GATT_CACHE_UNCACHED_ONLY
        );
        assert_eq!(
            GattCachePolicy::UncachedFirst.as_u8(),
            GATT_CACHE_UNCACHED_FIRST
        );
        assert_eq!(
            GattCachePolicy::CachedFirst.as_u8(),
            GATT_CACHE_CACHED_FIRST
        );
    }

    #[test]
    fn policy_attempt_sequences_are_ordered() {
        assert_eq!(
            GattCachePolicy::CachedOnly.modes(),
            &[GattCacheMode::Cached]
        );
        assert_eq!(
            GattCachePolicy::UncachedOnly.modes(),
            &[GattCacheMode::Uncached]
        );
        assert_eq!(
            GattCachePolicy::UncachedFirst.modes(),
            &[GattCacheMode::Uncached, GattCacheMode::Cached]
        );
        assert_eq!(
            GattCachePolicy::CachedFirst.modes(),
            &[GattCacheMode::Cached, GattCacheMode::Uncached]
        );
    }

    #[test]
    fn scenario_table_rows() {
        assert_eq!(
            RECENT_PAIRING_NOTIFY_CACHE_POLICY,
            GattCachePolicy::CachedFirst
        );
        assert_eq!(
            KNOWN_ADDRESS_NOTIFY_CACHE_POLICY,
            GattCachePolicy::UncachedFirst
        );
        assert_eq!(DEVICE_NOTIFY_CACHE_POLICY, GattCachePolicy::UncachedFirst);
        assert_eq!(
            PERSISTED_BOND_NOTIFY_CACHE_POLICY,
            GattCachePolicy::CachedFirst
        );
        assert_eq!(
            POST_CONFIRM_NOTIFY_CACHE_POLICY,
            GattCachePolicy::UncachedFirst
        );
        assert_eq!(
            SERVICE_REACHABILITY_PROBE_CACHE_POLICY,
            GattCachePolicy::CachedFirst
        );
        assert_eq!(STATUS_PROBE_CACHE_POLICY, GattCachePolicy::CachedFirst);
        assert_eq!(CONTROL_WRITE_CACHE_POLICY, GattCachePolicy::UncachedFirst);
        assert_eq!(DIAGNOSTIC_CACHE_POLICY, GattCachePolicy::UncachedOnly);
        assert_eq!(
            DEADLINE_SERVICE_ENDPOINT_CACHE_POLICY,
            GattCachePolicy::CachedFirst
        );
    }

    #[test]
    fn verified_handoff_forbids_cached_device_handles() {
        assert_eq!(
            ota_device_control_cache_policy(true),
            GattCachePolicy::UncachedOnly
        );
        assert_eq!(
            ota_device_control_cache_policy(false),
            GattCachePolicy::UncachedFirst
        );
    }

    #[test]
    fn unproven_service_endpoint_forbids_cached_handles() {
        assert_eq!(
            ota_service_endpoint_cache_policy(true),
            GattCachePolicy::UncachedFirst
        );
        assert_eq!(
            ota_service_endpoint_cache_policy(false),
            GattCachePolicy::UncachedOnly
        );
    }
}
