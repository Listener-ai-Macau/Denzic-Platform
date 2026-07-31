//! Portable post-AGC gain governor and paired frame statistics.

use crate::generated::{
    LEVELING_ATTENUATION_ATTACK_MS, LEVELING_ATTENUATION_RELEASE_MS,
    LEVELING_INITIAL_NOISE_FLOOR_MEAN_ABS, LEVELING_MAXIMUM_EFFECTIVE_GAIN_PERMILLE,
    LEVELING_MAXIMUM_NOISE_OUTPUT_MEAN_ABS, LEVELING_MINIMUM_NOISE_FLOOR_MEAN_ABS,
    LEVELING_NOISE_FALL_TIME_MS, LEVELING_NOISE_RISE_TIME_MS, LEVELING_SCALE_ONE_PERMILLE,
    LEVELING_SPEECH_HOLD_MS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub minimum_noise_floor_mean_abs: u32,
    pub initial_noise_floor_mean_abs: u32,
    pub noise_rise_time_ms: u32,
    pub noise_fall_time_ms: u32,
    pub speech_hold_ms: u32,
    pub attenuation_attack_ms: u32,
    pub attenuation_release_ms: u32,
    pub maximum_effective_gain_permille: u32,
    pub maximum_noise_output_mean_abs: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            minimum_noise_floor_mean_abs: LEVELING_MINIMUM_NOISE_FLOOR_MEAN_ABS,
            initial_noise_floor_mean_abs: LEVELING_INITIAL_NOISE_FLOOR_MEAN_ABS,
            noise_rise_time_ms: LEVELING_NOISE_RISE_TIME_MS,
            noise_fall_time_ms: LEVELING_NOISE_FALL_TIME_MS,
            speech_hold_ms: LEVELING_SPEECH_HOLD_MS,
            attenuation_attack_ms: LEVELING_ATTENUATION_ATTACK_MS,
            attenuation_release_ms: LEVELING_ATTENUATION_RELEASE_MS,
            maximum_effective_gain_permille: LEVELING_MAXIMUM_EFFECTIVE_GAIN_PERMILLE,
            maximum_noise_output_mean_abs: LEVELING_MAXIMUM_NOISE_OUTPUT_MEAN_ABS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input {
    pub raw_mean_abs: u32,
    pub post_agc_mean_abs: u32,
    pub elapsed_ms: u32,
    pub speech_detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub scale_permille: u32,
    pub noise_floor_mean_abs: u32,
    pub effective_gain_permille: u32,
    pub allowed_gain_permille: u32,
    pub speech_hold_remaining_ms: u32,
    pub speech_held: bool,
    pub gain_limited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    noise_floor_mean_abs: u32,
    scale_permille: u32,
    speech_hold_remaining_ms: u32,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            noise_floor_mean_abs: config.initial_noise_floor_mean_abs,
            scale_permille: LEVELING_SCALE_ONE_PERMILLE,
            speech_hold_remaining_ms: 0,
        }
    }

    pub fn step(&mut self, config: Config, input: Input) -> Output {
        if input.speech_detected {
            self.speech_hold_remaining_ms = config.speech_hold_ms;
        } else {
            self.speech_hold_remaining_ms = self
                .speech_hold_remaining_ms
                .saturating_sub(input.elapsed_ms);
        }
        let speech_held = input.speech_detected || self.speech_hold_remaining_ms > 0;

        if !speech_held {
            let target = input.raw_mean_abs.max(config.minimum_noise_floor_mean_abs);
            let transition_ms = if target > self.noise_floor_mean_abs {
                config.noise_rise_time_ms
            } else {
                config.noise_fall_time_ms
            };
            self.noise_floor_mean_abs = step_toward(
                self.noise_floor_mean_abs,
                target,
                input.elapsed_ms,
                transition_ms,
            )
            .max(config.minimum_noise_floor_mean_abs);
        }

        let effective_gain_permille =
            ratio_permille(input.post_agc_mean_abs, input.raw_mean_abs.max(1));
        let mut allowed_gain_permille = config.maximum_effective_gain_permille;
        if !speech_held {
            let noise_allowed = ratio_permille(
                config.maximum_noise_output_mean_abs,
                self.noise_floor_mean_abs.max(1),
            )
            .max(LEVELING_SCALE_ONE_PERMILLE);
            allowed_gain_permille = allowed_gain_permille.min(noise_allowed);
        }
        let mut desired_scale = if effective_gain_permille > allowed_gain_permille {
            ratio_permille(allowed_gain_permille, effective_gain_permille)
        } else {
            LEVELING_SCALE_ONE_PERMILLE
        };
        if speech_held && !input.speech_detected {
            desired_scale = self.scale_permille;
        }
        let transition_ms = if desired_scale < self.scale_permille {
            config.attenuation_attack_ms
        } else {
            config.attenuation_release_ms
        };
        self.scale_permille = step_toward(
            self.scale_permille,
            desired_scale,
            input.elapsed_ms,
            transition_ms,
        )
        .min(LEVELING_SCALE_ONE_PERMILLE);

        Output {
            scale_permille: self.scale_permille,
            noise_floor_mean_abs: self.noise_floor_mean_abs,
            effective_gain_permille,
            allowed_gain_permille,
            speech_hold_remaining_ms: self.speech_hold_remaining_ms,
            speech_held,
            gain_limited: self.scale_permille < LEVELING_SCALE_ONE_PERMILLE,
        }
    }
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    ((u64::from(numerator) * u64::from(LEVELING_SCALE_ONE_PERMILLE))
        / u64::from(denominator.max(1)))
    .min(u64::from(u32::MAX)) as u32
}

fn step_toward(current: u32, target: u32, elapsed_ms: u32, transition_ms: u32) -> u32 {
    if current == target || elapsed_ms == 0 {
        return current;
    }
    if transition_ms == 0 || elapsed_ms >= transition_ms {
        return target;
    }
    let distance = current.abs_diff(target);
    let amount = ((u64::from(distance) * u64::from(elapsed_ms) + u64::from(transition_ms - 1))
        / u64::from(transition_ms))
    .max(1)
    .min(u64::from(distance)) as u32;
    if current > target {
        current - amount
    } else {
        current + amount
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameObservation {
    pub input: Input,
    pub output: Output,
    pub limiter_applied: bool,
    pub clipped_samples: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PairedLevelStats {
    raw_mean_abs: Vec<u32>,
    post_agc_mean_abs: Vec<u32>,
    effective_gain_permille: Vec<u32>,
    voiced_raw_mean_abs: Vec<u32>,
    voiced_post_agc_mean_abs: Vec<u32>,
    voiced_effective_gain_permille: Vec<u32>,
    voiced_frames: u32,
    gain_limited_frames: u32,
    gain_ceiling_frames: u32,
    limiter_frames: u32,
    clipped_samples: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedLevelSummary {
    pub frames: u32,
    pub voiced_frames: u32,
    pub gain_limited_frames: u32,
    pub gain_ceiling_frames: u32,
    pub limiter_frames: u32,
    pub clipped_samples: u64,
    pub raw_mean_abs_p10: u32,
    pub raw_mean_abs_p50: u32,
    pub raw_mean_abs_p90: u32,
    pub post_agc_mean_abs_p10: u32,
    pub post_agc_mean_abs_p50: u32,
    pub post_agc_mean_abs_p90: u32,
    pub effective_gain_permille_p10: u32,
    pub effective_gain_permille_p50: u32,
    pub effective_gain_permille_p90: u32,
    pub voiced_raw_mean_abs_p10: u32,
    pub voiced_raw_mean_abs_p50: u32,
    pub voiced_raw_mean_abs_p90: u32,
    pub voiced_post_agc_mean_abs_p10: u32,
    pub voiced_post_agc_mean_abs_p50: u32,
    pub voiced_post_agc_mean_abs_p90: u32,
    pub voiced_effective_gain_permille_p10: u32,
    pub voiced_effective_gain_permille_p50: u32,
    pub voiced_effective_gain_permille_p90: u32,
}

impl PairedLevelStats {
    pub fn observe(&mut self, observation: FrameObservation) {
        self.raw_mean_abs.push(observation.input.raw_mean_abs);
        self.post_agc_mean_abs
            .push(observation.input.post_agc_mean_abs);
        self.effective_gain_permille
            .push(observation.output.effective_gain_permille);
        if observation.input.speech_detected {
            self.voiced_raw_mean_abs
                .push(observation.input.raw_mean_abs);
            self.voiced_post_agc_mean_abs
                .push(observation.input.post_agc_mean_abs);
            self.voiced_effective_gain_permille
                .push(observation.output.effective_gain_permille);
        }
        self.voiced_frames += u32::from(observation.input.speech_detected);
        self.gain_limited_frames += u32::from(observation.output.gain_limited);
        self.gain_ceiling_frames += u32::from(
            observation.output.effective_gain_permille >= observation.output.allowed_gain_permille,
        );
        self.limiter_frames += u32::from(observation.limiter_applied);
        self.clipped_samples += u64::from(observation.clipped_samples);
    }

    pub fn summarize(&self) -> PairedLevelSummary {
        PairedLevelSummary {
            frames: self.raw_mean_abs.len().min(u32::MAX as usize) as u32,
            voiced_frames: self.voiced_frames,
            gain_limited_frames: self.gain_limited_frames,
            gain_ceiling_frames: self.gain_ceiling_frames,
            limiter_frames: self.limiter_frames,
            clipped_samples: self.clipped_samples,
            raw_mean_abs_p10: percentile(&self.raw_mean_abs, 10),
            raw_mean_abs_p50: percentile(&self.raw_mean_abs, 50),
            raw_mean_abs_p90: percentile(&self.raw_mean_abs, 90),
            post_agc_mean_abs_p10: percentile(&self.post_agc_mean_abs, 10),
            post_agc_mean_abs_p50: percentile(&self.post_agc_mean_abs, 50),
            post_agc_mean_abs_p90: percentile(&self.post_agc_mean_abs, 90),
            effective_gain_permille_p10: percentile(&self.effective_gain_permille, 10),
            effective_gain_permille_p50: percentile(&self.effective_gain_permille, 50),
            effective_gain_permille_p90: percentile(&self.effective_gain_permille, 90),
            voiced_raw_mean_abs_p10: percentile(&self.voiced_raw_mean_abs, 10),
            voiced_raw_mean_abs_p50: percentile(&self.voiced_raw_mean_abs, 50),
            voiced_raw_mean_abs_p90: percentile(&self.voiced_raw_mean_abs, 90),
            voiced_post_agc_mean_abs_p10: percentile(&self.voiced_post_agc_mean_abs, 10),
            voiced_post_agc_mean_abs_p50: percentile(&self.voiced_post_agc_mean_abs, 50),
            voiced_post_agc_mean_abs_p90: percentile(&self.voiced_post_agc_mean_abs, 90),
            voiced_effective_gain_permille_p10: percentile(
                &self.voiced_effective_gain_permille,
                10,
            ),
            voiced_effective_gain_permille_p50: percentile(
                &self.voiced_effective_gain_permille,
                50,
            ),
            voiced_effective_gain_permille_p90: percentile(
                &self.voiced_effective_gain_permille,
                90,
            ),
        }
    }
}

fn percentile(values: &[u32], percent: usize) -> u32 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * percent).div_ceil(100).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(raw: u32, post: u32, speech: bool) -> Input {
        Input {
            raw_mean_abs: raw,
            post_agc_mean_abs: post,
            elapsed_ms: 10,
            speech_detected: speech,
        }
    }

    #[test]
    fn noisy_non_speech_is_bounded_but_speech_releases_and_holds() {
        let config = Config::default();
        let mut state = State::new(config);
        let mut output = state.step(config, input(16, 400, false));
        for _ in 1..500 {
            output = state.step(config, input(16, 400, false));
        }
        assert!(output.noise_floor_mean_abs >= 14);
        assert!(output.scale_permille < LEVELING_SCALE_ONE_PERMILLE);
        let quiet_scale = output.scale_permille;

        output = state.step(config, input(20, 360, true));
        assert!(output.speech_held);
        assert!(output.scale_permille > quiet_scale);
        for _ in 0..20 {
            output = state.step(config, input(20, 360, true));
        }
        assert_eq!(output.scale_permille, LEVELING_SCALE_ONE_PERMILLE);
        for _ in 0..20 {
            output = state.step(config, input(16, 400, false));
        }
        assert!(output.speech_held);
        assert_eq!(output.scale_permille, LEVELING_SCALE_ONE_PERMILLE);
    }

    #[test]
    fn paired_summary_reports_same_frame_pre_post_and_gain_percentiles() {
        let config = Config::default();
        let mut state = State::new(config);
        let mut stats = PairedLevelStats::default();
        for (raw, post, speech) in [(4, 40, false), (16, 240, true), (32, 320, true)] {
            let input = input(raw, post, speech);
            let output = state.step(config, input);
            stats.observe(FrameObservation {
                input,
                output,
                limiter_applied: false,
                clipped_samples: 0,
            });
        }
        let summary = stats.summarize();
        assert_eq!(summary.frames, 3);
        assert_eq!(summary.voiced_frames, 2);
        assert_eq!(summary.raw_mean_abs_p50, 16);
        assert_eq!(summary.post_agc_mean_abs_p90, 320);
        assert_eq!(summary.voiced_raw_mean_abs_p10, 16);
        assert_eq!(summary.voiced_raw_mean_abs_p90, 32);
        assert_eq!(summary.voiced_post_agc_mean_abs_p50, 240);
        assert_eq!(summary.voiced_effective_gain_permille_p90, 15_000);
        assert_eq!(summary.clipped_samples, 0);
    }
}
