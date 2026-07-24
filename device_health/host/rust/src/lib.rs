//! Product-independent boot safety, startup-check, and runtime-health policy.

mod generated;

pub use generated::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BootState {
    pub crash_count: u32,
    pub safe_mode_latched: bool,
}

impl BootState {
    pub fn observe(&mut self, reason: ResetReason, threshold: u32) {
        if !reset_counts_as_crash(reason) {
            *self = Self::default();
            return;
        }
        self.crash_count = self.crash_count.saturating_add(1);
        if threshold > 0 && self.crash_count >= threshold {
            self.safe_mode_latched = true;
        }
    }
}

pub fn reset_counts_as_crash(reason: ResetReason) -> bool {
    !matches!(
        reason,
        ResetReason::PowerOn
            | ResetReason::External
            | ResetReason::DeepSleep
            | ResetReason::Usb
            | ResetReason::Jtag
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub state: CheckState,
    pub critical: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelfTestSummary {
    pub total: u16,
    pub failed: u16,
    pub critical_failed: u16,
    pub ready: bool,
}

pub fn summarize_checks(checks: &[Check]) -> SelfTestSummary {
    let mut summary = SelfTestSummary {
        ready: true,
        ..SelfTestSummary::default()
    };
    for check in checks {
        summary.total = summary.total.saturating_add(1);
        if check.state == CheckState::Failed {
            summary.failed = summary.failed.saturating_add(1);
            if check.critical {
                summary.critical_failed = summary.critical_failed.saturating_add(1);
                summary.ready = false;
            }
        }
    }
    summary
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeInput {
    pub free_heap_kb: u32,
    pub heap_warn_kb: u32,
    pub disconnect_count: u32,
    pub previous_disconnect_count: u32,
    pub disconnect_rate_limit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDecision {
    pub alert: RuntimeAlert,
    pub observed: u32,
    pub threshold: u32,
    pub warning: bool,
}

pub fn evaluate_runtime(input: RuntimeInput) -> RuntimeDecision {
    if input.heap_warn_kb > 0 && input.free_heap_kb < input.heap_warn_kb {
        return RuntimeDecision {
            alert: RuntimeAlert::HeapPressure,
            observed: input.free_heap_kb,
            threshold: input.heap_warn_kb,
            warning: true,
        };
    }
    let disconnect_delta = input
        .disconnect_count
        .wrapping_sub(input.previous_disconnect_count);
    if disconnect_delta > input.disconnect_rate_limit {
        return RuntimeDecision {
            alert: RuntimeAlert::LinkChurn,
            observed: disconnect_delta,
            threshold: input.disconnect_rate_limit,
            warning: true,
        };
    }
    RuntimeDecision {
        alert: RuntimeAlert::None,
        observed: 0,
        threshold: 0,
        warning: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_state_latches_after_three_crashes_and_clears_on_power_on() {
        let mut state = BootState::default();
        state.observe(ResetReason::Panic, 3);
        state.observe(ResetReason::Watchdog, 3);
        state.observe(ResetReason::Software, 3);
        assert_eq!(state.crash_count, 3);
        assert!(state.safe_mode_latched);
        state.observe(ResetReason::PowerOn, 3);
        assert_eq!(state, BootState::default());
    }

    #[test]
    fn only_failed_critical_checks_block_readiness() {
        let summary = summarize_checks(&[
            Check {
                state: CheckState::Recovered,
                critical: true,
            },
            Check {
                state: CheckState::Failed,
                critical: false,
            },
        ]);
        assert!(summary.ready);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn heap_pressure_has_priority_over_link_churn() {
        let decision = evaluate_runtime(RuntimeInput {
            free_heap_kb: 19,
            heap_warn_kb: 20,
            disconnect_count: 5,
            previous_disconnect_count: 0,
            disconnect_rate_limit: 2,
        });
        assert_eq!(decision.alert, RuntimeAlert::HeapPressure);
    }
}
