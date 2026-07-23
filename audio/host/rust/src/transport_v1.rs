//! VKA1 audio stream transport mirror.
//!
//! Host-side counterpart of the embedded `denzic_audio_transport_v1` core:
//! both ends share the wire constants generated from
//! `audio/protocol/audio_transport_v1.json`, and the pacing, backpressure,
//! epoch, and replay-window semantics mirror the firmware decisions so host
//! tooling and replay tests observe the link exactly the way the device
//! drives it. Receive-side session reassembly and statistics live in
//! [`SessionCollector`](crate::SessionCollector); this module adds the
//! transport decisions around it.

use crate::SessionCollector;

pub use crate::generated_transport::{
    AUDIO_POOL_BUFFER_BYTES, AUDIO_POOL_EXTRA_DEFAULT, AUDIO_POOL_EXTRA_SPIRAM,
    AUDIO_POOL_WARN_PERCENT, BACKPRESSURE_PAUSE_PERCENT, BACKPRESSURE_RESUME_PERCENT,
    FIXED_SESSION_DURATION_SECONDS, FIXED_SESSION_PCM_BYTES, NOTIFY_QUEUE_LENGTH_DEFAULT,
    NOTIFY_QUEUE_LENGTH_SPIRAM, PACING_BYTES_PER_TICK, PACING_TARGET_BYTES_PER_SECOND,
    PACING_TICK_MS, PACKET_DEFAULT_VALUE_MAX_BYTES, PACKET_MAX_VALUE_BYTES,
    PACKET_MTU_ATT_OVERHEAD_BYTES, REPLAY_PAYLOAD_BYTES, REPLAY_WINDOW_PACKETS,
    TRANSPORT_PCM_BYTES_PER_SECOND, TRANSPORT_PROTOCOL_NAME, TRANSPORT_PROTOCOL_VERSION,
};

/// Media-clock pacing debt against the 38.4 kB/s wire target, in 10 ms ticks.
///
/// Mirrors `denzic_audio_transport_v1_pacing_t`: debt is tracked in source
/// PCM bytes and the sub-tick remainder carries over so the long-run rate
/// matches `PACING_BYTES_PER_TICK`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Pacing {
    debt_bytes: u32,
}

impl Pacing {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn debt_bytes(&self) -> u32 {
        self.debt_bytes
    }

    /// Accounts one successfully submitted audio packet and returns how many
    /// pacing ticks the sender must wait before continuing.
    pub fn note_pcm_sent(&mut self, packet_pcm_bytes: u32) -> u32 {
        self.debt_bytes += packet_pcm_bytes;
        let delay_ticks = self.debt_bytes / PACING_BYTES_PER_TICK;
        self.debt_bytes %= PACING_BYTES_PER_TICK;
        delay_ticks
    }
}

/// Pressure is the larger of queue and pool occupancy, in percent.
pub fn pressure_percent(
    queue_depth: u32,
    queue_capacity: u32,
    pool_in_use: u32,
    pool_capacity: u32,
) -> u32 {
    let queue_percent = if queue_capacity > 0 {
        queue_depth * 100 / queue_capacity
    } else {
        0
    };
    let pool_percent = if pool_capacity > 0 {
        pool_in_use * 100 / pool_capacity
    } else {
        0
    };
    queue_percent.max(pool_percent)
}

/// Backpressure hysteresis from flow_control_v1.md section 3: without an
/// active session capture never stays paused; pressure >= pause% pauses;
/// pressure <= resume% resumes; between the thresholds the state holds.
pub fn backpressure_decide(pause_active: bool, session_active: bool, pressure: u32) -> bool {
    if !session_active {
        return false;
    }
    if pressure >= BACKPRESSURE_PAUSE_PERCENT {
        return true;
    }
    if pressure <= BACKPRESSURE_RESUME_PERCENT {
        return false;
    }
    pause_active
}

/// Pool occupancy level (in buffers) at which the once-per-session pressure
/// warning fires; the latch stays in the adapter.
pub fn pool_warn_level(pool_capacity: u32) -> u32 {
    (pool_capacity * AUDIO_POOL_WARN_PERCENT + 99) / 100
}

/// Maps an exchanged ATT MTU to the notify value byte budget: MTU minus the
/// ATT overhead, clamped to `[header_bytes, PACKET_MAX_VALUE_BYTES]`. Returns
/// `None` when the MTU carries no ATT payload and the previous value holds.
pub fn packet_value_max_from_mtu(mtu: u16, header_bytes: u16) -> Option<u16> {
    if mtu <= PACKET_MTU_ATT_OVERHEAD_BYTES {
        return None;
    }
    let value_max = mtu - PACKET_MTU_ATT_OVERHEAD_BYTES;
    Some(value_max.clamp(header_bytes, PACKET_MAX_VALUE_BYTES))
}

/// Next connection epoch value; zero is reserved and skipped on wrap.
pub fn next_epoch(current_epoch: u32) -> u32 {
    match current_epoch.wrapping_add(1) {
        0 => 1,
        next => next,
    }
}

/// flow_control_v1.md section 2: a subscribe/MTU/notify_tx event applies only
/// when both the connection handle and the epoch captured at issue time match
/// the current link; anything else is stale and may only be counted.
pub fn event_applies(
    current_conn_handle: u16,
    current_epoch: u32,
    event_conn_handle: u16,
    event_epoch: u32,
) -> bool {
    event_conn_handle == current_conn_handle && event_epoch == current_epoch
}

/// Disconnect events carry no epoch; only the connection handle decides.
pub fn link_event_applies(current_conn_handle: u16, event_conn_handle: u16) -> bool {
    event_conn_handle == current_conn_handle
}

/// One retained replay packet; flags — including the lossless Rice flag — are
/// preserved verbatim for resend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayPacket {
    pub session_id: u32,
    pub sequence: u16,
    pub payload: Vec<u8>,
    pub packet_pcm_bytes: u16,
    pub flags: u8,
}

/// Outcome of [`ReplayWindow::store`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStoreResult {
    Ignored,
    New,
    Replaced,
}

/// Host mirror of the device-side 48-packet replay window: packets are
/// retained until notify success removes them, retaining a 49th packet evicts
/// the oldest, and a re-retained sequence replaces the previous copy.
#[derive(Debug)]
pub struct ReplayWindow {
    packets: [Option<ReplayPacket>; REPLAY_WINDOW_PACKETS],
    next_index: u32,
    pending: bool,
    pending_session_id: u32,
    in_progress: bool,
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self {
            packets: std::array::from_fn(|_| None),
            next_index: 0,
            pending: false,
            pending_session_id: 0,
            in_progress: false,
        }
    }
}

impl ReplayWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn pending_session_id(&self) -> u32 {
        self.pending_session_id
    }

    pub fn in_progress(&self) -> bool {
        self.in_progress
    }

    pub fn store(
        &mut self,
        session_id: u32,
        sequence: u16,
        payload: &[u8],
        packet_pcm_bytes: u16,
        flags: u8,
    ) -> ReplayStoreResult {
        if session_id == 0
            || payload.is_empty()
            || payload.len() > REPLAY_PAYLOAD_BYTES
            || self.in_progress
        {
            return ReplayStoreResult::Ignored;
        }

        for slot in &mut self.packets {
            if let Some(existing) = slot {
                if existing.session_id == session_id && existing.sequence == sequence {
                    existing.payload = payload.to_vec();
                    existing.packet_pcm_bytes = packet_pcm_bytes;
                    existing.flags = flags;
                    return ReplayStoreResult::Replaced;
                }
            }
        }

        let slot = &mut self.packets[self.next_index as usize % REPLAY_WINDOW_PACKETS];
        *slot = Some(ReplayPacket {
            session_id,
            sequence,
            payload: payload.to_vec(),
            packet_pcm_bytes,
            flags,
        });
        self.next_index = self.next_index.wrapping_add(1);
        ReplayStoreResult::New
    }

    /// Drops a retained packet after notify success; returns true when found.
    pub fn remove(&mut self, session_id: u32, sequence: u16) -> bool {
        if session_id == 0 {
            return false;
        }
        for slot in &mut self.packets {
            let matches = slot.as_ref().is_some_and(|packet| {
                packet.session_id == session_id && packet.sequence == sequence
            });
            if matches {
                *slot = None;
                return true;
            }
        }
        false
    }

    /// Purges every retained packet of a session and dismisses its pending
    /// replay; used on cancel and on terminal session teardown.
    pub fn clear_session(&mut self, session_id: u32) {
        if session_id == 0 {
            return;
        }
        for slot in &mut self.packets {
            let matches = slot
                .as_ref()
                .is_some_and(|packet| packet.session_id == session_id);
            if matches {
                *slot = None;
            }
        }
        if self.pending && self.pending_session_id == session_id {
            self.pending = false;
            self.pending_session_id = 0;
        }
    }

    pub fn count_retained(&self, session_id: u32) -> u32 {
        self.packets
            .iter()
            .flatten()
            .filter(|packet| packet.session_id == session_id)
            .count() as u32
    }

    /// Arms the retained window as pending replay after link loss or
    /// notify-disabled with an active session. Returns false (and leaves the
    /// window untouched) when nothing is retained.
    pub fn mark_suspended(&mut self, session_id: u32) -> bool {
        if session_id == 0 || self.count_retained(session_id) == 0 {
            return false;
        }
        self.pending = true;
        self.pending_session_id = session_id;
        true
    }

    /// Collects the pending packets of a session in ascending sequence order
    /// for resend. When `skip_current_sequence` is set, the packet the live
    /// path is about to send is left out; the second return value reports the
    /// omission. Does not mutate window state.
    pub fn collect_pending(
        &self,
        session_id: u32,
        skip_current_sequence: bool,
        current_sequence: u16,
    ) -> (Vec<&ReplayPacket>, bool) {
        if self.in_progress || !self.pending || self.pending_session_id != session_id {
            return (Vec::new(), false);
        }
        let mut skipped = false;
        let mut drain: Vec<&ReplayPacket> = Vec::new();
        for packet in self.packets.iter().flatten() {
            if packet.session_id != session_id {
                continue;
            }
            if skip_current_sequence && packet.sequence == current_sequence {
                skipped = true;
                continue;
            }
            drain.push(packet);
        }
        drain.sort_by_key(|packet| packet.sequence);
        (drain, skipped)
    }

    /// Marks resend in progress; while set, [`store`](Self::store) ignores
    /// new packets.
    pub fn begin_drain(&mut self) {
        self.in_progress = true;
    }

    /// All pending packets resent: leaves drain mode and disarms the window.
    pub fn complete_drain(&mut self) {
        self.in_progress = false;
        self.pending = false;
        self.pending_session_id = 0;
    }

    /// A resend failed: leaves drain mode but keeps the window pending so the
    /// retained packets are retried through the normal notify retry path.
    pub fn abort_drain(&mut self) {
        self.in_progress = false;
    }

    /// Nothing left to resend for the pending session: disarms the window.
    pub fn dismiss_pending(&mut self) {
        self.pending = false;
        self.pending_session_id = 0;
    }
}

/// True while a session is open and no terminal packet arrived; such a
/// session may still recover after a link outage through the replay window.
pub fn has_active_recoverable_session(collector: &SessionCollector) -> bool {
    collector.session_id().is_some() && !collector.terminal_received()
}

/// flow_control_v1.md section 5: when the rolling stop-drain idle window
/// expires, a session that stopped with audio finalizes successfully; any
/// other outcome is a capture failure whose reason the adapter formats.
pub fn stop_drain_expired_finalizes(collector: &SessionCollector) -> bool {
    collector.has_stopped_with_audio()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{collect_notifications, ReplayConfig};

    #[test]
    fn pacing_accumulates_debt_across_packets() {
        let mut pacing = Pacing::new();
        assert_eq!(pacing.note_pcm_sent(480), 1);
        assert_eq!(pacing.note_pcm_sent(480), 1);
        assert_eq!(pacing.note_pcm_sent(480), 1);
        assert_eq!(pacing.note_pcm_sent(480), 2);

        let mut pacing = Pacing::new();
        assert_eq!(pacing.note_pcm_sent(100), 0);
        assert_eq!(pacing.note_pcm_sent(284), 1);
        assert_eq!(pacing.note_pcm_sent(0), 0);

        let mut pacing = Pacing::new();
        let total_ticks: u32 = (0..100).map(|_| pacing.note_pcm_sent(320)).sum();
        assert_eq!(total_ticks, 83);
        assert_eq!(pacing.debt_bytes(), 128);
    }

    #[test]
    fn pressure_percent_uses_true_capacity_max() {
        assert_eq!(pressure_percent(24, 48, 0, 52), 50);
        assert_eq!(pressure_percent(0, 48, 26, 52), 50);
        assert_eq!(pressure_percent(47, 48, 10, 52), 97);
        assert_eq!(pressure_percent(0, 0, 0, 0), 0);
    }

    #[test]
    fn backpressure_hysteresis_holds_inside_the_band() {
        assert!(!backpressure_decide(true, false, 100));
        let mut pause = backpressure_decide(false, true, 95);
        assert!(pause);
        pause = backpressure_decide(pause, true, 71);
        assert!(pause);
        pause = backpressure_decide(pause, true, 70);
        assert!(!pause);
        pause = backpressure_decide(pause, true, 94);
        assert!(!pause);
        assert!(backpressure_decide(pause, true, 100));
    }

    #[test]
    fn pool_warn_level_rounds_up() {
        assert_eq!(pool_warn_level(52), 42);
        assert_eq!(pool_warn_level(264), 212);
        assert_eq!(pool_warn_level(0), 0);
    }

    #[test]
    fn packet_value_max_from_mtu_clamps_to_budget() {
        assert_eq!(packet_value_max_from_mtu(0, 20), None);
        assert_eq!(packet_value_max_from_mtu(3, 20), None);
        assert_eq!(packet_value_max_from_mtu(4, 20), Some(20));
        assert_eq!(packet_value_max_from_mtu(247, 20), Some(244));
        assert_eq!(packet_value_max_from_mtu(517, 20), Some(500));
        assert_eq!(packet_value_max_from_mtu(600, 20), Some(500));
    }

    #[test]
    fn epoch_advance_skips_zero() {
        assert_eq!(next_epoch(0), 1);
        assert_eq!(next_epoch(41), 42);
        assert_eq!(next_epoch(u32::MAX), 1);
    }

    #[test]
    fn stale_event_classification_matches_contract() {
        assert!(event_applies(1, 9, 1, 9));
        assert!(!event_applies(1, 9, 2, 9));
        assert!(!event_applies(1, 9, 1, 8));
        assert!(link_event_applies(1, 1));
        assert!(!link_event_applies(1, 2));
    }

    #[test]
    fn replay_window_retains_replaces_and_evicts_oldest() {
        let mut window = ReplayWindow::new();
        assert_eq!(
            window.store(0, 0, &[1; 10], 10, 0),
            ReplayStoreResult::Ignored
        );
        assert_eq!(window.store(7, 0, &[], 0, 0), ReplayStoreResult::Ignored);
        assert_eq!(
            window.store(7, 5, &[3; 100], 96, 0x01),
            ReplayStoreResult::New
        );
        assert_eq!(window.count_retained(7), 1);

        assert_eq!(
            window.store(7, 5, &[9; 40], 40, 0x01),
            ReplayStoreResult::Replaced
        );
        assert_eq!(window.count_retained(7), 1);

        for sequence in 100..148u16 {
            assert_eq!(
                window.store(7, sequence, &[1; 10], 10, 0),
                ReplayStoreResult::New
            );
        }
        assert_eq!(window.count_retained(7), REPLAY_WINDOW_PACKETS as u32);
        assert!(!window.remove(7, 5));
        assert!(window.remove(7, 147));
    }

    #[test]
    fn replay_window_drains_ascending_and_skips_current() {
        let mut window = ReplayWindow::new();
        for sequence in [12u16, 10, 11] {
            window.store(7, sequence, &[4; 8], 8, 0);
        }
        window.store(99, 1, &[4; 8], 8, 0);

        let (drain, skipped) = window.collect_pending(7, false, 0);
        assert!(drain.is_empty());
        assert!(!skipped);

        assert!(window.mark_suspended(7));
        let (drain, skipped) = window.collect_pending(7, false, 0);
        assert_eq!(
            drain
                .iter()
                .map(|packet| packet.sequence)
                .collect::<Vec<_>>(),
            vec![10, 11, 12]
        );
        assert!(!skipped);

        let (drain, skipped) = window.collect_pending(7, true, 11);
        assert_eq!(
            drain
                .iter()
                .map(|packet| packet.sequence)
                .collect::<Vec<_>>(),
            vec![10, 12]
        );
        assert!(skipped);
        assert_eq!(window.count_retained(7), 3);
    }

    #[test]
    fn replay_window_drain_outcomes() {
        let mut window = ReplayWindow::new();
        window.store(7, 0, &[5; 8], 8, 0);
        assert!(window.mark_suspended(7));

        window.begin_drain();
        assert_eq!(
            window.store(7, 1, &[5; 8], 8, 0),
            ReplayStoreResult::Ignored
        );
        assert!(window.collect_pending(7, false, 0).0.is_empty());

        window.abort_drain();
        assert!(!window.in_progress());
        assert!(window.pending());
        assert_eq!(window.pending_session_id(), 7);

        window.begin_drain();
        window.complete_drain();
        assert!(!window.pending());
        assert_eq!(window.count_retained(7), 1);

        assert!(window.mark_suspended(7));
        window.dismiss_pending();
        assert!(!window.pending());
        assert_eq!(window.pending_session_id(), 0);
    }

    #[test]
    fn recoverable_session_and_stop_drain_outcome() {
        let config = ReplayConfig {
            session_id: 3,
            payload_pcm_bytes: 32,
        };
        let pcm = vec![7u8; 64];
        let notifications =
            crate::build_session_replay_notifications(config, &pcm).expect("replay build");

        let running = collect_notifications(
            notifications[..notifications.len() - 1]
                .iter()
                .map(Vec::as_slice),
        )
        .expect("collect running");
        assert!(has_active_recoverable_session(&running));
        assert!(!stop_drain_expired_finalizes(&running));

        let stopped = collect_notifications(notifications.iter().map(Vec::as_slice))
            .expect("collect stopped");
        assert!(!has_active_recoverable_session(&stopped));
        assert!(stop_drain_expired_finalizes(&stopped));
    }
}
