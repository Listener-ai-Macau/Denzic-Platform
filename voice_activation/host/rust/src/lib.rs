//! Product-independent speech-triggered recording state machine.

mod generated;

pub use generated::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub speech_confirm_ms: u32,
    pub pre_roll_ms: u32,
    pub silence_stop_ms: u32,
    pub tail_ms: u32,
    pub min_session_ms: u32,
    pub max_session_ms: u32,
    pub cooldown_ms: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            speech_confirm_ms: DEFAULT_SPEECH_CONFIRM_MS,
            pre_roll_ms: DEFAULT_PRE_ROLL_MS,
            silence_stop_ms: DEFAULT_SILENCE_STOP_MS,
            tail_ms: DEFAULT_TAIL_MS,
            min_session_ms: DEFAULT_MIN_SESSION_MS,
            max_session_ms: DEFAULT_MAX_SESSION_MS,
            cooldown_ms: DEFAULT_COOLDOWN_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input {
    pub elapsed_ms: u32,
    pub enabled: bool,
    pub auto_start_enabled: bool,
    pub auto_stop_enabled: bool,
    pub recording_active: bool,
    pub speech_detected: bool,
    pub start_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub state: State,
    pub action: Action,
    pub stop_reason: StopReason,
    pub pre_roll_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateInput {
    pub phrase_signal: PhraseSignal,
    pub owner_match: Option<bool>,
    pub terminal: bool,
}

pub fn decide_gate(input: GateInput) -> GateDecision {
    if input.phrase_signal != PhraseSignal::None {
        return match input.owner_match {
            Some(true) => GateDecision::Accept,
            Some(false) => GateDecision::Reject,
            None if input.terminal => GateDecision::Reject,
            None => GateDecision::Pending,
        };
    }
    if input.terminal {
        GateDecision::Reject
    } else {
        GateDecision::Pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Machine {
    state: State,
    speech_ms: u32,
    silence_ms: u32,
    recording_ms: u32,
    cooldown_ms: u32,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            state: State::Disabled,
            speech_ms: 0,
            silence_ms: 0,
            recording_ms: 0,
            cooldown_ms: 0,
        }
    }
}

fn add_saturating(value: &mut u32, elapsed_ms: u32) {
    *value = value.saturating_add(elapsed_ms);
}

impl Machine {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn step(&mut self, config: Config, input: Input) -> Decision {
        let mut decision = Decision {
            state: self.state,
            action: Action::None,
            stop_reason: StopReason::None,
            pre_roll_ms: 0,
        };
        if !input.enabled || (!input.auto_start_enabled && !input.auto_stop_enabled) {
            self.reset();
            decision.state = State::Disabled;
            return decision;
        }

        if input.recording_active {
            if self.state != State::Recording && self.state != State::Tail {
                self.recording_ms = 0;
                self.silence_ms = 0;
            }
            add_saturating(&mut self.recording_ms, input.elapsed_ms);
            self.speech_ms = 0;
            if input.speech_detected {
                self.silence_ms = 0;
                self.state = State::Recording;
            } else if input.auto_stop_enabled {
                add_saturating(&mut self.silence_ms, input.elapsed_ms);
                self.state = if self.silence_ms >= config.silence_stop_ms {
                    State::Tail
                } else {
                    State::Recording
                };
            } else {
                self.silence_ms = 0;
                self.state = State::Recording;
            }

            if input.auto_stop_enabled
                && config.max_session_ms > 0
                && self.recording_ms >= config.max_session_ms
            {
                decision.action = Action::Stop;
                decision.stop_reason = StopReason::MaxDuration;
            } else if input.auto_stop_enabled
                && self.recording_ms >= config.min_session_ms
                && self.silence_ms >= config.silence_stop_ms.saturating_add(config.tail_ms)
            {
                decision.action = Action::Stop;
                decision.stop_reason = StopReason::Silence;
            }
            decision.state = self.state;
            return decision;
        }

        if self.state == State::Recording || self.state == State::Tail {
            self.state = State::Cooldown;
            self.cooldown_ms = config.cooldown_ms;
            self.recording_ms = 0;
            self.silence_ms = 0;
        }
        if self.state == State::Cooldown {
            self.cooldown_ms = self.cooldown_ms.saturating_sub(input.elapsed_ms);
            if self.cooldown_ms > 0 {
                decision.state = State::Cooldown;
                return decision;
            }
            self.state = State::Monitoring;
        }

        self.recording_ms = 0;
        self.silence_ms = 0;
        if !input.auto_start_enabled || input.start_blocked || !input.speech_detected {
            self.speech_ms = 0;
            self.state = State::Monitoring;
        } else {
            add_saturating(&mut self.speech_ms, input.elapsed_ms);
            self.state = State::SpeechConfirming;
            if self.speech_ms >= config.speech_confirm_ms {
                decision.action = Action::Start;
                decision.pre_roll_ms = config.pre_roll_ms;
                self.speech_ms = 0;
            }
        }
        decision.state = self.state;
        decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(speech: bool) -> Input {
        Input {
            elapsed_ms: 100,
            enabled: true,
            auto_start_enabled: true,
            auto_stop_enabled: true,
            recording_active: false,
            speech_detected: speech,
            start_blocked: false,
        }
    }

    #[test]
    fn noise_or_short_speech_does_not_start() {
        let mut machine = Machine::default();
        assert_eq!(
            machine.step(Config::default(), input(false)).action,
            Action::None
        );
        assert_eq!(
            machine.step(Config::default(), input(true)).action,
            Action::None
        );
        assert_eq!(
            machine.step(Config::default(), input(false)).action,
            Action::None
        );
        assert_eq!(machine.state(), State::Monitoring);
    }

    #[test]
    fn confirmed_speech_starts_with_real_preroll_request() {
        let mut machine = Machine::default();
        assert_eq!(
            machine.step(Config::default(), input(true)).action,
            Action::None
        );
        assert_eq!(
            machine.step(Config::default(), input(true)).action,
            Action::None
        );
        let decision = machine.step(Config::default(), input(true));
        assert_eq!(decision.action, Action::Start);
        assert_eq!(decision.pre_roll_ms, 600);
    }

    #[test]
    fn stop_waits_for_silence_and_tail_and_respects_minimum() {
        let mut machine = Machine::default();
        let mut active = input(false);
        active.recording_active = true;
        for _ in 0..12 {
            assert_eq!(machine.step(Config::default(), active).action, Action::None);
        }
        assert_eq!(
            machine.step(Config::default(), active).stop_reason,
            StopReason::Silence
        );
    }

    #[test]
    fn speech_during_tail_resumes_recording() {
        let mut machine = Machine::default();
        let mut active = input(false);
        active.recording_active = true;
        for _ in 0..10 {
            machine.step(Config::default(), active);
        }
        active.speech_detected = true;
        let decision = machine.step(Config::default(), active);
        assert_eq!(decision.state, State::Recording);
        assert_eq!(decision.action, Action::None);
    }

    #[test]
    fn maximum_duration_stops_even_during_speech() {
        let mut machine = Machine::default();
        let config = Config {
            max_session_ms: 300,
            ..Config::default()
        };
        let mut active = input(true);
        active.recording_active = true;
        machine.step(config, active);
        machine.step(config, active);
        assert_eq!(
            machine.step(config, active).stop_reason,
            StopReason::MaxDuration
        );
    }

    #[test]
    fn cooldown_prevents_immediate_restart() {
        let mut machine = Machine::default();
        let mut active = input(true);
        active.recording_active = true;
        machine.step(Config::default(), active);
        active.recording_active = false;
        active.elapsed_ms = 100;
        assert_eq!(
            machine.step(Config::default(), active).state,
            State::Cooldown
        );
        active.elapsed_ms = 1400;
        assert_eq!(
            machine.step(Config::default(), active).state,
            State::SpeechConfirming
        );
    }

    #[test]
    fn wake_gate_accepts_either_local_phrase_signal_only_with_owner_match() {
        for phrase_signal in [PhraseSignal::KeywordModel, PhraseSignal::LocalTranscript] {
            assert_eq!(
                decide_gate(GateInput {
                    phrase_signal,
                    owner_match: Some(true),
                    terminal: false,
                }),
                GateDecision::Accept
            );
            assert_eq!(
                decide_gate(GateInput {
                    phrase_signal,
                    owner_match: Some(false),
                    terminal: false,
                }),
                GateDecision::Reject
            );
        }
    }

    #[test]
    fn wake_gate_stays_pending_until_a_signal_or_terminal_boundary() {
        assert_eq!(
            decide_gate(GateInput {
                phrase_signal: PhraseSignal::None,
                owner_match: None,
                terminal: false,
            }),
            GateDecision::Pending
        );
        assert_eq!(
            decide_gate(GateInput {
                phrase_signal: PhraseSignal::None,
                owner_match: Some(true),
                terminal: true,
            }),
            GateDecision::Reject
        );
        assert_eq!(
            decide_gate(GateInput {
                phrase_signal: PhraseSignal::LocalTranscript,
                owner_match: None,
                terminal: true,
            }),
            GateDecision::Reject
        );
    }
}
