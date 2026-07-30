//! Product-independent speech-triggered recording state machine.

mod generated;

use pinyin::ToPinyin;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalPhraseRelation {
    ExactStart,
    PhoneticStart,
    PresentLater,
    Absent,
}

fn normalized_phrase_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn phonetic_phrase_units(value: &str) -> Vec<String> {
    normalized_phrase_text(value)
        .chars()
        .map(|ch| {
            ch.to_pinyin()
                .map(|value| value.plain().to_string())
                .unwrap_or_else(|| ch.to_lowercase().collect())
        })
        .collect()
}

pub fn local_transcript_phrase_relation(transcript: &str, phrase: &str) -> LocalPhraseRelation {
    let phrase = normalized_phrase_text(phrase);
    if phrase.is_empty() {
        return LocalPhraseRelation::Absent;
    }
    let transcript = normalized_phrase_text(transcript);
    if transcript.starts_with(&phrase) {
        LocalPhraseRelation::ExactStart
    } else {
        let phrase_units = phonetic_phrase_units(&phrase);
        let transcript_units = phonetic_phrase_units(&transcript);
        if transcript_units.len() >= phrase_units.len()
            && transcript_units[..phrase_units.len()] == phrase_units
        {
            LocalPhraseRelation::PhoneticStart
        } else if transcript.contains(&phrase) {
            LocalPhraseRelation::PresentLater
        } else {
            LocalPhraseRelation::Absent
        }
    }
}

pub fn local_transcript_matches_phrase(transcript: &str, phrase: &str) -> bool {
    matches!(
        local_transcript_phrase_relation(transcript, phrase),
        LocalPhraseRelation::ExactStart | LocalPhraseRelation::PhoneticStart
    )
}

pub const fn confirmation_snapshot_ms(attempt: usize, snapshots_ms: &[usize]) -> Option<usize> {
    if attempt < snapshots_ms.len() {
        Some(snapshots_ms[attempt])
    } else {
        None
    }
}

pub fn pcm_offset_after_activation(
    end_seconds: f32,
    pad_seconds: f32,
    bytes_per_second: usize,
    pcm_len: usize,
    alignment: usize,
) -> usize {
    if !end_seconds.is_finite() || end_seconds <= 0.0 || pcm_len < alignment || alignment == 0 {
        return 0;
    }
    let offset = ((end_seconds + pad_seconds) * bytes_per_second as f32) as usize;
    let bounded = offset.min(pcm_len);
    bounded - bounded % alignment
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalConfirmationBoundaryInput {
    pub keyword_end_seconds: f32,
    pub recovered_keyword_end_seconds: Option<f32>,
    pub phrase_relation: LocalPhraseRelation,
    pub transcript_chars: usize,
    pub phrase_chars: usize,
    pub snapshot_pcm_ms: usize,
    pub end_pad_seconds: f32,
    pub local_endpoint_max_seconds: f32,
}

pub fn refined_local_wake_end_seconds(input: LocalConfirmationBoundaryInput) -> f32 {
    let model_boundary = input
        .recovered_keyword_end_seconds
        .filter(|seconds| {
            seconds.is_finite() && *seconds > 0.0 && *seconds <= input.local_endpoint_max_seconds
        })
        .unwrap_or_default();
    let detected_boundary = input.keyword_end_seconds.max(model_boundary);
    let start_aligned = matches!(
        input.phrase_relation,
        LocalPhraseRelation::ExactStart | LocalPhraseRelation::PhoneticStart
    );
    let exact_phrase_only = start_aligned && input.transcript_chars <= input.phrase_chars;
    if !exact_phrase_only {
        if detected_boundary > 0.0 {
            return detected_boundary;
        }
        if !start_aligned || input.phrase_chars == 0 || input.transcript_chars <= input.phrase_chars
        {
            return 0.0;
        }
        let snapshot_seconds = input.snapshot_pcm_ms as f32 / 1_000.0;
        let proportional =
            snapshot_seconds * input.phrase_chars as f32 / input.transcript_chars as f32;
        return proportional
            .clamp(0.55, input.local_endpoint_max_seconds)
            .min((snapshot_seconds - input.end_pad_seconds).max(0.0));
    }
    let local_phrase_end = input.snapshot_pcm_ms as f32 / 1_000.0 - input.end_pad_seconds;
    detected_boundary.max(local_phrase_end.max(0.0))
}

pub const fn local_confirmation_can_activate(
    has_keyword_model_hit: bool,
    relation: LocalPhraseRelation,
) -> bool {
    if has_keyword_model_hit {
        matches!(
            relation,
            LocalPhraseRelation::ExactStart
                | LocalPhraseRelation::PhoneticStart
                | LocalPhraseRelation::PresentLater
        )
    } else {
        matches!(
            relation,
            LocalPhraseRelation::ExactStart | LocalPhraseRelation::PhoneticStart
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondaryFallbackInput {
    pub keyword_model_hit: bool,
    pub explicit_absent_count: u8,
    pub secondary_unavailable_or_timed_out: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryFallbackDecision {
    HoldForConfirmation,
    AcceptKeywordModel,
}

/// Decide whether a high-recall keyword hit may bypass an unavailable
/// precision verifier. Any explicit absence is authoritative and prevents a
/// later timeout or helper failure from reversing that evidence.
pub const fn decide_secondary_fallback(input: SecondaryFallbackInput) -> SecondaryFallbackDecision {
    if input.keyword_model_hit
        && input.secondary_unavailable_or_timed_out
        && input.explicit_absent_count == 0
    {
        SecondaryFallbackDecision::AcceptKeywordModel
    } else {
        SecondaryFallbackDecision::HoldForConfirmation
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

    #[test]
    fn secondary_fallback_accepts_only_before_explicit_absence() {
        assert_eq!(
            decide_secondary_fallback(SecondaryFallbackInput {
                keyword_model_hit: true,
                explicit_absent_count: 0,
                secondary_unavailable_or_timed_out: true,
            }),
            SecondaryFallbackDecision::AcceptKeywordModel
        );
        assert_eq!(
            decide_secondary_fallback(SecondaryFallbackInput {
                keyword_model_hit: true,
                explicit_absent_count: 1,
                secondary_unavailable_or_timed_out: true,
            }),
            SecondaryFallbackDecision::HoldForConfirmation
        );
    }

    #[test]
    fn secondary_fallback_does_not_accept_without_failure_or_keyword_hit() {
        for input in [
            SecondaryFallbackInput {
                keyword_model_hit: false,
                explicit_absent_count: 0,
                secondary_unavailable_or_timed_out: true,
            },
            SecondaryFallbackInput {
                keyword_model_hit: true,
                explicit_absent_count: 0,
                secondary_unavailable_or_timed_out: false,
            },
        ] {
            assert_eq!(
                decide_secondary_fallback(input),
                SecondaryFallbackDecision::HoldForConfirmation
            );
        }
    }

    #[test]
    fn transcript_relation_requires_a_start_aligned_phrase_for_direct_match() {
        assert_eq!(
            local_transcript_phrase_relation("开始录音，今天测试。", "开始录音"),
            LocalPhraseRelation::ExactStart
        );
        assert_eq!(
            local_transcript_phrase_relation("开使录因，今天测试", "开始录音"),
            LocalPhraseRelation::PhoneticStart
        );
        assert_eq!(
            local_transcript_phrase_relation("请说开始录音", "开始录音"),
            LocalPhraseRelation::PresentLater
        );
        assert_eq!(
            local_transcript_phrase_relation("普通说话", "开始录音"),
            LocalPhraseRelation::Absent
        );
        assert!(local_transcript_matches_phrase(
            "开始录音，今天测试。",
            "开始录音"
        ));
        assert!(!local_transcript_matches_phrase("请说开始录音", "开始录音"));
    }

    #[test]
    fn confirmation_schedule_and_pcm_boundary_are_product_configured() {
        assert_eq!(
            confirmation_snapshot_ms(1, &[1_000, 1_400, 1_800]),
            Some(1_400)
        );
        assert_eq!(confirmation_snapshot_ms(3, &[1_000, 1_400, 1_800]), None);
        assert_eq!(
            pcm_offset_after_activation(1.0, 0.12, 32_000, 40_001, 2),
            35_840
        );
    }

    #[test]
    fn local_wake_boundary_preserves_start_and_body_rules() {
        let phrase_only = LocalConfirmationBoundaryInput {
            keyword_end_seconds: 0.685,
            recovered_keyword_end_seconds: None,
            phrase_relation: LocalPhraseRelation::ExactStart,
            transcript_chars: 4,
            phrase_chars: 4,
            snapshot_pcm_ms: 1_800,
            end_pad_seconds: 0.12,
            local_endpoint_max_seconds: 1.20,
        };
        assert!((refined_local_wake_end_seconds(phrase_only) - 1.68).abs() < 0.001);
        assert_eq!(
            refined_local_wake_end_seconds(LocalConfirmationBoundaryInput {
                keyword_end_seconds: 0.0,
                recovered_keyword_end_seconds: None,
                phrase_relation: LocalPhraseRelation::PresentLater,
                transcript_chars: 9,
                ..phrase_only
            }),
            0.0
        );
        assert!(local_confirmation_can_activate(
            true,
            LocalPhraseRelation::PresentLater
        ));
        assert!(!local_confirmation_can_activate(
            false,
            LocalPhraseRelation::PresentLater
        ));
    }
}
