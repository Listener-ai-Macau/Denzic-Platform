//! Product-independent low-power blocker and idle/shutdown decisions.

mod generated;

pub use generated::*;

pub const fn blocker_bit(blocker: Blocker) -> u32 {
    1u32 << blocker as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub connected_idle_ms: u32,
    pub disconnected_idle_ms: u32,
    pub shutdown_idle_ms: u32,
    pub sleep_enabled: bool,
    pub shutdown_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input {
    pub sleep_blockers: u32,
    pub shutdown_blockers: u32,
    pub user_idle_ms: u32,
    pub radio_idle_ms: u32,
    pub connected: bool,
    pub shutdown_retry_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub state: State,
    pub shutdown_reason: ShutdownReason,
    pub sleep_allowed: bool,
    pub shutdown_allowed: bool,
}

pub fn evaluate(config: Config, input: Input) -> Decision {
    if config.shutdown_enabled
        && config.shutdown_idle_ms > 0
        && input.user_idle_ms >= config.shutdown_idle_ms
        && input.shutdown_blockers == 0
        && !input.shutdown_retry_active
    {
        return Decision {
            state: State::ShutdownRequested,
            shutdown_reason: ShutdownReason::LongIdle,
            sleep_allowed: false,
            shutdown_allowed: true,
        };
    }
    let threshold = if input.connected {
        config.connected_idle_ms
    } else {
        config.disconnected_idle_ms
    };
    if config.sleep_enabled
        && input.sleep_blockers == 0
        && threshold > 0
        && input.radio_idle_ms >= threshold
    {
        return Decision {
            state: if input.connected {
                State::ConnectedIdle
            } else {
                State::DisconnectedIdle
            },
            shutdown_reason: ShutdownReason::None,
            sleep_allowed: true,
            shutdown_allowed: false,
        };
    }
    Decision {
        state: State::Active,
        shutdown_reason: ShutdownReason::None,
        sleep_allowed: false,
        shutdown_allowed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            connected_idle_ms: 100,
            disconnected_idle_ms: 200,
            shutdown_idle_ms: 1000,
            sleep_enabled: true,
            shutdown_enabled: true,
        }
    }

    #[test]
    fn blockers_keep_the_product_active() {
        let decision = evaluate(
            config(),
            Input {
                sleep_blockers: blocker_bit(Blocker::Recording),
                shutdown_blockers: 0,
                user_idle_ms: 0,
                radio_idle_ms: 100,
                connected: true,
                shutdown_retry_active: false,
            },
        );
        assert_eq!(decision.state, State::Active);
    }

    #[test]
    fn connected_and_disconnected_thresholds_are_distinct() {
        let mut input = Input {
            sleep_blockers: 0,
            shutdown_blockers: 0,
            user_idle_ms: 0,
            radio_idle_ms: 100,
            connected: true,
            shutdown_retry_active: false,
        };
        assert_eq!(evaluate(config(), input).state, State::ConnectedIdle);
        input.connected = false;
        assert_eq!(evaluate(config(), input).state, State::Active);
    }

    #[test]
    fn shutdown_requires_no_shutdown_blockers_or_retry() {
        let decision = evaluate(
            config(),
            Input {
                sleep_blockers: 0,
                shutdown_blockers: 0,
                user_idle_ms: 1000,
                radio_idle_ms: 1000,
                connected: false,
                shutdown_retry_active: false,
            },
        );
        assert!(decision.shutdown_allowed);
        assert_eq!(decision.shutdown_reason, ShutdownReason::LongIdle);
    }
}
