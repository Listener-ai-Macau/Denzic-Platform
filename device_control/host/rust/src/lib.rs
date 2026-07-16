//! Transport-neutral device connection and control state machine.

mod generated;

pub use generated::*;

pub const TERMINAL_HISTORY_CAPACITY: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationRequest {
    pub operation_id: u64,
    pub idempotency_key: u64,
    pub target_id: u32,
    pub expected_settings_revision: u32,
    pub timeout_ms: u32,
    pub kind: OperationKind,
    pub automatic: bool,
    pub recovery_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationDecision {
    pub result: OperationResult,
    pub error: ErrorCategory,
    pub execute: bool,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalRecord {
    pub operation_id: u64,
    pub idempotency_key: u64,
    pub kind: OperationKind,
    pub result: OperationResult,
    pub error: ErrorCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveOperation {
    request: OperationRequest,
    deadline_ms: u32,
    write_acknowledged: bool,
    write_acknowledged_revision: u32,
}

#[derive(Debug, Clone)]
pub struct DeviceControlCore {
    pub transport: Transport,
    pub lifecycle: LifecycleState,
    pub ownership: OwnershipState,
    pub capability_mask: u64,
    pub capability_revision: u32,
    pub settings_revision: u32,
    active: Option<ActiveOperation>,
    terminal_history: [Option<TerminalRecord>; TERMINAL_HISTORY_CAPACITY],
    terminal_history_next: usize,
}

impl DeviceControlCore {
    pub const fn new(transport: Transport) -> Self {
        Self {
            transport,
            lifecycle: LifecycleState::Disconnected,
            ownership: OwnershipState::Unknown,
            capability_mask: 0,
            capability_revision: 0,
            settings_revision: 0,
            active: None,
            terminal_history: [None; TERMINAL_HISTORY_CAPACITY],
            terminal_history_next: 0,
        }
    }

    pub fn set_lifecycle(&mut self, lifecycle: LifecycleState) {
        if lifecycle == LifecycleState::OwnedElsewhere {
            self.set_ownership(OwnershipState::External);
        } else if self.ownership != OwnershipState::External {
            self.lifecycle = lifecycle;
        }
    }

    pub fn set_ownership(&mut self, ownership: OwnershipState) {
        self.ownership = ownership;
        match ownership {
            OwnershipState::External => {
                self.lifecycle = LifecycleState::OwnedElsewhere;
                self.finish_active(OperationResult::Failed, ErrorCategory::Ownership);
            }
            OwnershipState::ManualUnpaired => {
                self.lifecycle = LifecycleState::Disconnected;
                self.finish_active(OperationResult::Cancelled, ErrorCategory::Ownership);
            }
            _ => {}
        }
    }

    pub fn begin(&mut self, request: OperationRequest, now_ms: u32) -> OperationDecision {
        if request.operation_id == 0
            || request.idempotency_key == 0
            || request.timeout_ms == 0
            || request.kind == OperationKind::Unknown
        {
            return Self::reject(ErrorCategory::Protocol);
        }
        if let Some(active) = self.active {
            if Self::same_request(request, active.request) {
                return Self::decision(OperationResult::Accepted, ErrorCategory::None, false, true);
            }
            return Self::reject(ErrorCategory::Resource);
        }
        if let Some(record) = self.terminal(request.operation_id, request.idempotency_key) {
            return Self::replay(record);
        }
        if self.ownership == OwnershipState::External
            || self.lifecycle == LifecycleState::OwnedElsewhere
        {
            return Self::reject(ErrorCategory::Ownership);
        }
        if request.automatic
            && (self.ownership == OwnershipState::ManualUnpaired
                || (!request.recovery_authorized && self.ownership != OwnershipState::Local))
        {
            return Self::reject(ErrorCategory::Ownership);
        }
        if !Self::lifecycle_accepts(self.lifecycle, request.kind) {
            return Self::reject(if self.lifecycle == LifecycleState::Failed {
                ErrorCategory::Device
            } else {
                ErrorCategory::Protocol
            });
        }
        self.active = Some(ActiveOperation {
            request,
            deadline_ms: now_ms.wrapping_add(request.timeout_ms),
            write_acknowledged: false,
            write_acknowledged_revision: 0,
        });
        Self::decision(OperationResult::Accepted, ErrorCategory::None, true, false)
    }

    pub fn cancel(&mut self, operation_id: u64, idempotency_key: u64) -> OperationDecision {
        if self.active_matches(operation_id, idempotency_key) {
            self.finish_active(OperationResult::Cancelled, ErrorCategory::None);
            return Self::decision(
                OperationResult::Cancelled,
                ErrorCategory::None,
                false,
                false,
            );
        }
        self.terminal(operation_id, idempotency_key)
            .map(Self::replay)
            .unwrap_or_else(|| Self::reject(ErrorCategory::Protocol))
    }

    pub fn complete(
        &mut self,
        operation_id: u64,
        idempotency_key: u64,
        result: OperationResult,
        error: ErrorCategory,
    ) -> OperationDecision {
        if !self.active_matches(operation_id, idempotency_key) {
            return self
                .terminal(operation_id, idempotency_key)
                .map(Self::replay)
                .unwrap_or_else(|| Self::reject(ErrorCategory::Protocol));
        }
        if !matches!(
            result,
            OperationResult::Succeeded
                | OperationResult::Cancelled
                | OperationResult::TimedOut
                | OperationResult::TransportLost
                | OperationResult::Failed
        ) {
            return Self::reject(ErrorCategory::Protocol);
        }
        if self.active.unwrap().request.kind == OperationKind::WriteSetting
            && result == OperationResult::Succeeded
        {
            return Self::reject(ErrorCategory::Protocol);
        }
        self.finish_active(result, error);
        Self::decision(result, error, false, false)
    }

    pub fn expire(&mut self, now_ms: u32) -> bool {
        let Some(active) = self.active else {
            return false;
        };
        if (now_ms.wrapping_sub(active.deadline_ms) as i32) < 0 {
            return false;
        }
        self.finish_active(OperationResult::TimedOut, ErrorCategory::Timeout);
        true
    }

    pub fn report_transport_lost(&mut self) -> bool {
        let had_active = self.active.is_some();
        self.lifecycle = LifecycleState::Disconnected;
        self.finish_active(OperationResult::TransportLost, ErrorCategory::Transport);
        had_active
    }

    pub fn report_capabilities(
        &mut self,
        operation_id: u64,
        idempotency_key: u64,
        revision: u32,
        capability_mask: u64,
    ) -> bool {
        if !self.active_matches(operation_id, idempotency_key)
            || self.active.unwrap().request.kind != OperationKind::ReadCapabilities
        {
            return false;
        }
        self.capability_revision = revision;
        self.capability_mask = capability_mask;
        self.finish_active(OperationResult::Succeeded, ErrorCategory::None);
        true
    }

    pub fn acknowledge_setting_write(
        &mut self,
        operation_id: u64,
        idempotency_key: u64,
        acknowledged_revision: u32,
    ) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        if !Self::matches(
            operation_id,
            idempotency_key,
            active.request.operation_id,
            active.request.idempotency_key,
        ) || active.request.kind != OperationKind::WriteSetting
            || acknowledged_revision < active.request.expected_settings_revision
        {
            return false;
        }
        active.write_acknowledged = true;
        active.write_acknowledged_revision = acknowledged_revision;
        true
    }

    pub fn confirm_setting_readback(
        &mut self,
        operation_id: u64,
        idempotency_key: u64,
        observed_revision: u32,
    ) -> bool {
        let Some(active) = self.active else {
            return false;
        };
        if !Self::matches(
            operation_id,
            idempotency_key,
            active.request.operation_id,
            active.request.idempotency_key,
        ) || !matches!(
            active.request.kind,
            OperationKind::ReadSetting | OperationKind::WriteSetting
        ) || observed_revision < self.settings_revision
            || (active.request.kind == OperationKind::WriteSetting
                && (!active.write_acknowledged
                    || observed_revision < active.write_acknowledged_revision))
        {
            return false;
        }
        self.settings_revision = observed_revision;
        self.finish_active(OperationResult::Succeeded, ErrorCategory::None);
        true
    }

    pub fn active_operation(&self) -> Option<OperationRequest> {
        self.active.map(|active| active.request)
    }

    fn same_request(left: OperationRequest, right: OperationRequest) -> bool {
        Self::matches(
            left.operation_id,
            left.idempotency_key,
            right.operation_id,
            right.idempotency_key,
        )
    }

    fn matches(
        operation_id: u64,
        idempotency_key: u64,
        candidate_id: u64,
        candidate_key: u64,
    ) -> bool {
        operation_id == candidate_id && idempotency_key == candidate_key
    }

    fn active_matches(&self, operation_id: u64, idempotency_key: u64) -> bool {
        self.active
            .map(|active| {
                Self::matches(
                    operation_id,
                    idempotency_key,
                    active.request.operation_id,
                    active.request.idempotency_key,
                )
            })
            .unwrap_or(false)
    }

    fn terminal(&self, operation_id: u64, idempotency_key: u64) -> Option<TerminalRecord> {
        self.terminal_history.into_iter().flatten().find(|record| {
            Self::matches(
                operation_id,
                idempotency_key,
                record.operation_id,
                record.idempotency_key,
            )
        })
    }

    fn finish_active(&mut self, result: OperationResult, error: ErrorCategory) {
        let Some(active) = self.active.take() else {
            return;
        };
        self.terminal_history[self.terminal_history_next] = Some(TerminalRecord {
            operation_id: active.request.operation_id,
            idempotency_key: active.request.idempotency_key,
            kind: active.request.kind,
            result,
            error,
        });
        self.terminal_history_next = (self.terminal_history_next + 1) % TERMINAL_HISTORY_CAPACITY;
    }

    fn lifecycle_accepts(lifecycle: LifecycleState, kind: OperationKind) -> bool {
        match kind {
            OperationKind::Discover => matches!(
                lifecycle,
                LifecycleState::Disconnected | LifecycleState::Recovering
            ),
            OperationKind::Connect => matches!(
                lifecycle,
                LifecycleState::Disconnected
                    | LifecycleState::Discovering
                    | LifecycleState::Recovering
            ),
            OperationKind::Secure => matches!(
                lifecycle,
                LifecycleState::Connecting | LifecycleState::Securing
            ),
            OperationKind::ReadCapabilities
            | OperationKind::ReadSetting
            | OperationKind::WriteSetting
            | OperationKind::InvokeCommand => lifecycle == LifecycleState::Ready,
            OperationKind::Unknown => false,
        }
    }

    const fn decision(
        result: OperationResult,
        error: ErrorCategory,
        execute: bool,
        replayed: bool,
    ) -> OperationDecision {
        OperationDecision {
            result,
            error,
            execute,
            replayed,
        }
    }

    const fn reject(error: ErrorCategory) -> OperationDecision {
        Self::decision(OperationResult::Rejected, error, false, false)
    }

    const fn replay(record: TerminalRecord) -> OperationDecision {
        Self::decision(record.result, record.error, false, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(operation_id: u64, idempotency_key: u64, kind: OperationKind) -> OperationRequest {
        OperationRequest {
            operation_id,
            idempotency_key,
            target_id: 17,
            expected_settings_revision: 0,
            timeout_ms: 20,
            kind,
            automatic: false,
            recovery_authorized: false,
        }
    }

    fn ready_core() -> DeviceControlCore {
        let mut core = DeviceControlCore::new(Transport::Ble);
        core.set_ownership(OwnershipState::Local);
        core.set_lifecycle(LifecycleState::Ready);
        core
    }

    #[test]
    fn duplicate_request_executes_once_and_replays_terminal_result() {
        let mut core = ready_core();
        let value = request(1, 101, OperationKind::InvokeCommand);
        assert!(core.begin(value, 10).execute);
        let duplicate = core.begin(value, 11);
        assert_eq!(duplicate.result, OperationResult::Accepted);
        assert!(duplicate.replayed);
        assert!(!duplicate.execute);
        core.complete(1, 101, OperationResult::Succeeded, ErrorCategory::None);
        let replay = core.begin(value, 12);
        assert_eq!(replay.result, OperationResult::Succeeded);
        assert!(replay.replayed);
        assert!(!replay.execute);
    }

    #[test]
    fn settings_write_requires_acknowledged_readback() {
        let mut core = ready_core();
        let mut value = request(2, 102, OperationKind::WriteSetting);
        value.expected_settings_revision = 7;
        assert!(core.begin(value, 20).execute);
        assert_eq!(
            core.complete(2, 102, OperationResult::Succeeded, ErrorCategory::None)
                .result,
            OperationResult::Rejected
        );
        assert!(core.acknowledge_setting_write(2, 102, 8));
        assert!(!core.confirm_setting_readback(2, 102, 7));
        assert!(core.confirm_setting_readback(2, 102, 8));
        assert_eq!(core.settings_revision, 8);
    }

    #[test]
    fn manual_unpair_stays_passive_and_external_owner_stops_recovery() {
        let mut core = DeviceControlCore::new(Transport::Ble);
        core.set_ownership(OwnershipState::ManualUnpaired);
        let mut automatic = request(3, 103, OperationKind::Connect);
        automatic.automatic = true;
        automatic.recovery_authorized = true;
        let rejected = core.begin(automatic, 30);
        assert_eq!(rejected.result, OperationResult::Rejected);
        assert_eq!(rejected.error, ErrorCategory::Ownership);

        let mut core = ready_core();
        let value = request(4, 104, OperationKind::InvokeCommand);
        assert!(core.begin(value, 40).execute);
        core.set_ownership(OwnershipState::External);
        assert_eq!(core.lifecycle, LifecycleState::OwnedElsewhere);
        let replay = core.begin(value, 41);
        assert_eq!(replay.result, OperationResult::Failed);
        assert_eq!(replay.error, ErrorCategory::Ownership);
        assert!(replay.replayed);
    }

    #[test]
    fn capability_readback_and_timeouts_have_terminal_evidence() {
        let mut core = ready_core();
        let capability = request(5, 105, OperationKind::ReadCapabilities);
        assert!(core.begin(capability, 50).execute);
        assert!(core.report_capabilities(5, 105, 9, 0xa5));
        assert_eq!(core.capability_revision, 9);
        assert_eq!(core.capability_mask, 0xa5);

        let mut timed = request(6, 106, OperationKind::InvokeCommand);
        timed.timeout_ms = 1;
        assert!(core.begin(timed, 60).execute);
        assert!(core.expire(61));
        let replay = core.begin(timed, 62);
        assert_eq!(replay.result, OperationResult::TimedOut);
        assert_eq!(replay.error, ErrorCategory::Timeout);
        assert!(replay.replayed);
    }
}
