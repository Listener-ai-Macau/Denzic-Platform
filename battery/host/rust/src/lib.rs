//! Product-independent battery level, charge, and notification policy.

mod generated;

pub use generated::*;

pub fn percent_from_mv(battery_mv: u32, empty_mv: u32, full_mv: u32) -> u8 {
    if full_mv <= empty_mv || battery_mv <= empty_mv {
        return 0;
    }
    if battery_mv >= full_mv {
        return 100;
    }
    let range = full_mv - empty_mv;
    (((battery_mv - empty_mv) * 100 + range / 2) / range) as u8
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChargeTracker {
    pub full_latched: bool,
    pub full_candidate_since_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChargeInput {
    pub charge_power_present: bool,
    pub battery_valid: bool,
    pub level_percent: u8,
    pub battery_mv: u32,
    pub raw_charging: bool,
    pub raw_full: bool,
    pub now_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChargeDecision {
    pub state: ChargeState,
    pub published_level: u8,
    pub charge_power_present: bool,
    pub full_latched: bool,
    pub full_candidate_ms: u32,
}

impl ChargeTracker {
    pub fn update(&mut self, input: ChargeInput) -> ChargeDecision {
        let power_present = input.charge_power_present || input.raw_full;
        if !power_present {
            *self = Self::default();
            return ChargeDecision {
                state: ChargeState::Discharging,
                published_level: input.level_percent.min(100),
                charge_power_present: false,
                full_latched: false,
                full_candidate_ms: 0,
            };
        }
        let allows_full = !input.battery_valid
            || input.battery_mv >= CHARGE_FULL_MIN_MV
            || u32::from(input.level_percent) >= CHARGE_FULL_MIN_PERCENT;
        let candidate = input.raw_full && !input.raw_charging && allows_full;
        if !self.full_latched {
            if !candidate {
                self.full_candidate_since_ms = 0;
            } else if self.full_candidate_since_ms == 0 {
                self.full_candidate_since_ms = input.now_ms;
            } else if input.now_ms.wrapping_sub(self.full_candidate_since_ms)
                >= CHARGE_FULL_DEBOUNCE_MS
            {
                self.full_latched = true;
            }
        }
        let mut level = input.level_percent.min(100);
        let state = if self.full_latched {
            level = 100;
            ChargeState::Full
        } else if input.raw_charging {
            level = level.min(99);
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        ChargeDecision {
            state,
            published_level: level,
            charge_power_present: power_present,
            full_latched: self.full_latched,
            full_candidate_ms: if self.full_candidate_since_ms == 0 {
                0
            } else {
                input.now_ms.wrapping_sub(self.full_candidate_since_ms)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotifyInput {
    pub previous_level: Option<u8>,
    pub level: u8,
    pub force: bool,
    pub now_ms: u32,
    pub last_notify_ms: u32,
    pub periodic_interval_ms: u32,
    pub threshold_percent: u8,
}

pub fn decide_notify(input: NotifyInput) -> NotifyReason {
    if input.force {
        return NotifyReason::Forced;
    }
    let Some(previous) = input
        .previous_level
        .filter(|level| u32::from(*level) != INVALID_LEVEL)
    else {
        return NotifyReason::Initial;
    };
    if input.level.abs_diff(previous) >= input.threshold_percent {
        return NotifyReason::LevelDelta;
    }
    if input.periodic_interval_ms > 0
        && input.now_ms.wrapping_sub(input.last_notify_ms) >= input.periodic_interval_ms
    {
        return NotifyReason::Periodic;
    }
    NotifyReason::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_reference_curve_keeps_midpoint() {
        assert_eq!(percent_from_mv(3500, 2850, 4150), 50);
    }

    #[test]
    fn full_requires_debounce_and_publishes_one_hundred() {
        let mut tracker = ChargeTracker::default();
        let mut input = ChargeInput {
            charge_power_present: true,
            battery_valid: true,
            level_percent: 90,
            battery_mv: 4100,
            raw_charging: false,
            raw_full: true,
            now_ms: 100,
        };
        assert!(!tracker.update(input).full_latched);
        input.now_ms = 10_101;
        let decision = tracker.update(input);
        assert!(decision.full_latched);
        assert_eq!(decision.published_level, 100);
    }

    #[test]
    fn notification_priority_is_force_delta_then_periodic() {
        assert_eq!(
            decide_notify(NotifyInput {
                previous_level: Some(50),
                level: 51,
                force: false,
                now_ms: 0,
                last_notify_ms: 0,
                periodic_interval_ms: 60_000,
                threshold_percent: 1,
            }),
            NotifyReason::LevelDelta
        );
    }
}
