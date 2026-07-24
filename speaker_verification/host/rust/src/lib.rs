//! Product-independent policy for gating provisional audio by speaker identity.

mod generated;

pub use generated::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input {
    pub enabled: bool,
    pub enrolled: bool,
    pub origin: CandidateOrigin,
    pub candidate_complete: bool,
    pub candidate_ms: u32,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub state: State,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Machine {
    state: State,
}

impl Default for Machine {
    fn default() -> Self {
        Self { state: State::Idle }
    }
}

impl Machine {
    pub fn reset(&mut self) {
        self.state = State::Idle;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn step(&mut self, input: Input) -> Decision {
        let terminal = |state, action| Decision { state, action };

        if input.origin == CandidateOrigin::Manual || !input.enabled {
            self.state = State::Released;
            return terminal(self.state, Action::Bypass);
        }
        if input.origin == CandidateOrigin::Automatic && !input.enrolled {
            self.state = State::Discarded;
            return terminal(self.state, Action::Discard);
        }
        if input.candidate_ms > DEFAULT_MAX_CANDIDATE_MS {
            self.state = State::Discarded;
            return terminal(self.state, Action::Discard);
        }
        if !input.candidate_complete {
            self.state = State::Buffering;
            return terminal(self.state, Action::Buffer);
        }
        if input.candidate_ms < DEFAULT_MIN_CANDIDATE_MS {
            self.state = State::Discarded;
            return terminal(self.state, Action::Discard);
        }

        match input.verdict {
            Verdict::Match => {
                self.state = State::Released;
                terminal(self.state, Action::Release)
            }
            Verdict::NonMatch => {
                self.state = State::Discarded;
                terminal(self.state, Action::Discard)
            }
            Verdict::Pending | Verdict::Unavailable => {
                self.state = State::Error;
                terminal(self.state, Action::Discard)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn automatic(verdict: Verdict) -> Input {
        Input {
            enabled: true,
            enrolled: true,
            origin: CandidateOrigin::Automatic,
            candidate_complete: true,
            candidate_ms: 2200,
            verdict,
        }
    }

    #[test]
    fn manual_recording_always_bypasses_identity_gate() {
        let decision = Machine::default().step(Input {
            origin: CandidateOrigin::Manual,
            ..automatic(Verdict::Pending)
        });
        assert_eq!(decision.action, Action::Bypass);
    }

    #[test]
    fn automatic_candidate_buffers_until_complete() {
        let mut machine = Machine::default();
        let decision = machine.step(Input {
            candidate_complete: false,
            candidate_ms: 400,
            ..automatic(Verdict::Pending)
        });
        assert_eq!(decision.state, State::Buffering);
        assert_eq!(decision.action, Action::Buffer);
    }

    #[test]
    fn matching_automatic_candidate_is_released() {
        let decision = Machine::default().step(automatic(Verdict::Match));
        assert_eq!(decision.action, Action::Release);
    }

    #[test]
    fn non_match_and_unavailable_runtime_fail_closed() {
        for verdict in [Verdict::NonMatch, Verdict::Unavailable] {
            let decision = Machine::default().step(automatic(verdict));
            assert_eq!(decision.action, Action::Discard);
        }
    }

    #[test]
    fn missing_enrollment_and_bad_duration_fail_closed() {
        let no_enrollment = Machine::default().step(Input {
            enrolled: false,
            ..automatic(Verdict::Match)
        });
        assert_eq!(no_enrollment.action, Action::Discard);

        for candidate_ms in [900, DEFAULT_MAX_CANDIDATE_MS + 1] {
            let decision = Machine::default().step(Input {
                candidate_ms,
                ..automatic(Verdict::Match)
            });
            assert_eq!(decision.action, Action::Discard);
        }
    }
}
