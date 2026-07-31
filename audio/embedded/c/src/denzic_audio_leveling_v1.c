#include "denzic_audio_leveling_v1.h"

#include <limits.h>
#include <string.h>

static uint32_t clamp_u64_to_u32(uint64_t value)
{
    return value > UINT32_MAX ? UINT32_MAX : (uint32_t)value;
}

static uint32_t step_toward(
    uint32_t current,
    uint32_t target,
    uint32_t elapsed_ms,
    uint32_t transition_ms)
{
    if (current == target || elapsed_ms == 0u) {
        return current;
    }
    if (transition_ms == 0u || elapsed_ms >= transition_ms) {
        return target;
    }
    uint32_t distance = current > target ? current - target : target - current;
    uint32_t amount = clamp_u64_to_u32(
        ((uint64_t)distance * elapsed_ms + transition_ms - 1u) / transition_ms);
    if (amount == 0u) {
        amount = 1u;
    }
    if (amount > distance) {
        amount = distance;
    }
    return current > target ? current - amount : current + amount;
}

static uint32_t effective_gain_permille(uint32_t input_mean, uint32_t output_mean)
{
    uint32_t denominator = input_mean == 0u ? 1u : input_mean;
    return clamp_u64_to_u32(
        ((uint64_t)output_mean * DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE) /
        denominator);
}

denzic_audio_leveling_v1_config_t denzic_audio_leveling_v1_default_config(void)
{
    denzic_audio_leveling_v1_config_t config = {
        .minimum_noise_floor_mean_abs = DENZIC_AUDIO_V1_LEVELING_MINIMUM_NOISE_FLOOR_MEAN_ABS,
        .initial_noise_floor_mean_abs = DENZIC_AUDIO_V1_LEVELING_INITIAL_NOISE_FLOOR_MEAN_ABS,
        .noise_rise_time_ms = DENZIC_AUDIO_V1_LEVELING_NOISE_RISE_TIME_MS,
        .noise_fall_time_ms = DENZIC_AUDIO_V1_LEVELING_NOISE_FALL_TIME_MS,
        .speech_hold_ms = DENZIC_AUDIO_V1_LEVELING_SPEECH_HOLD_MS,
        .attenuation_attack_ms = DENZIC_AUDIO_V1_LEVELING_ATTENUATION_ATTACK_MS,
        .attenuation_release_ms = DENZIC_AUDIO_V1_LEVELING_ATTENUATION_RELEASE_MS,
        .maximum_effective_gain_permille = DENZIC_AUDIO_V1_LEVELING_MAXIMUM_EFFECTIVE_GAIN_PERMILLE,
        .maximum_noise_output_mean_abs = DENZIC_AUDIO_V1_LEVELING_MAXIMUM_NOISE_OUTPUT_MEAN_ABS,
    };
    return config;
}

void denzic_audio_leveling_v1_reset(
    denzic_audio_leveling_v1_state_t *state,
    const denzic_audio_leveling_v1_config_t *config)
{
    if (state == NULL || config == NULL) {
        return;
    }
    state->noise_floor_q8 = config->initial_noise_floor_mean_abs << 8u;
    state->scale_permille = DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
    state->speech_hold_remaining_ms = 0u;
}

denzic_audio_leveling_v1_output_t denzic_audio_leveling_v1_step(
    denzic_audio_leveling_v1_state_t *state,
    const denzic_audio_leveling_v1_config_t *config,
    denzic_audio_leveling_v1_input_t input)
{
    denzic_audio_leveling_v1_output_t output = {0};
    if (state == NULL || config == NULL) {
        output.scale_permille = DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
        return output;
    }

    if (input.speech_detected) {
        state->speech_hold_remaining_ms = config->speech_hold_ms;
    } else if (state->speech_hold_remaining_ms > input.elapsed_ms) {
        state->speech_hold_remaining_ms -= input.elapsed_ms;
    } else {
        state->speech_hold_remaining_ms = 0u;
    }
    bool speech_held = input.speech_detected || state->speech_hold_remaining_ms > 0u;

    if (!speech_held && input.elapsed_ms > 0u) {
        uint32_t floor = state->noise_floor_q8 >> 8u;
        uint32_t target = input.raw_mean_abs;
        if (target < config->minimum_noise_floor_mean_abs) {
            target = config->minimum_noise_floor_mean_abs;
        }
        uint32_t transition_ms = target > floor
            ? config->noise_rise_time_ms
            : config->noise_fall_time_ms;
        floor = step_toward(floor, target, input.elapsed_ms, transition_ms);
        if (floor < config->minimum_noise_floor_mean_abs) {
            floor = config->minimum_noise_floor_mean_abs;
        }
        state->noise_floor_q8 = floor << 8u;
    }

    uint32_t noise_floor = state->noise_floor_q8 >> 8u;
    if (noise_floor < config->minimum_noise_floor_mean_abs) {
        noise_floor = config->minimum_noise_floor_mean_abs;
    }
    uint32_t effective_gain = effective_gain_permille(
        input.raw_mean_abs,
        input.post_agc_mean_abs);
    uint32_t allowed_gain = config->maximum_effective_gain_permille;
    if (!speech_held) {
        uint32_t noise_allowed = clamp_u64_to_u32(
            ((uint64_t)config->maximum_noise_output_mean_abs *
             DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE) /
            noise_floor);
        if (noise_allowed < DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE) {
            noise_allowed = DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
        }
        if (noise_allowed < allowed_gain) {
            allowed_gain = noise_allowed;
        }
    }

    uint32_t desired_scale = DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
    if (effective_gain > allowed_gain && effective_gain > 0u) {
        desired_scale = clamp_u64_to_u32(
            ((uint64_t)allowed_gain * DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE) /
            effective_gain);
    }
    if (speech_held && !input.speech_detected) {
        desired_scale = state->scale_permille;
    }
    uint32_t transition_ms = desired_scale < state->scale_permille
        ? config->attenuation_attack_ms
        : config->attenuation_release_ms;
    state->scale_permille = step_toward(
        state->scale_permille,
        desired_scale,
        input.elapsed_ms,
        transition_ms);
    if (state->scale_permille > DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE) {
        state->scale_permille = DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
    }

    output.scale_permille = state->scale_permille;
    output.noise_floor_mean_abs = noise_floor;
    output.effective_gain_permille = effective_gain;
    output.allowed_gain_permille = allowed_gain;
    output.speech_hold_remaining_ms = state->speech_hold_remaining_ms;
    output.speech_held = speech_held;
    output.gain_limited = state->scale_permille < DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
    return output;
}

static size_t level_bin(uint32_t value)
{
    size_t bin = 0u;
    while (value > 1u && bin + 1u < DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS) {
        value = (value + 1u) >> 1u;
        bin++;
    }
    return bin;
}

static size_t gain_bin(uint32_t gain_permille)
{
    uint32_t normalized = gain_permille /
        DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE;
    return level_bin(normalized == 0u ? 1u : normalized);
}

void denzic_audio_leveling_v1_stats_reset(
    denzic_audio_leveling_v1_stats_t *stats)
{
    if (stats != NULL) {
        memset(stats, 0, sizeof(*stats));
    }
}

void denzic_audio_leveling_v1_stats_observe(
    denzic_audio_leveling_v1_stats_t *stats,
    denzic_audio_leveling_v1_input_t input,
    denzic_audio_leveling_v1_output_t output,
    bool limiter_applied,
    uint32_t clipped_samples)
{
    if (stats == NULL) {
        return;
    }
    stats->frames++;
    stats->voiced_frames += input.speech_detected ? 1u : 0u;
    stats->gain_limited_frames += output.gain_limited ? 1u : 0u;
    stats->gain_ceiling_frames +=
        output.effective_gain_permille >= output.allowed_gain_permille ? 1u : 0u;
    stats->limiter_frames += limiter_applied ? 1u : 0u;
    stats->clipped_samples += clipped_samples;
    stats->raw_histogram[level_bin(input.raw_mean_abs)]++;
    stats->post_agc_histogram[level_bin(input.post_agc_mean_abs)]++;
    stats->effective_gain_histogram[gain_bin(output.effective_gain_permille)]++;
    if (input.speech_detected) {
        stats->voiced_raw_histogram[level_bin(input.raw_mean_abs)]++;
        stats->voiced_post_agc_histogram[level_bin(input.post_agc_mean_abs)]++;
        stats->voiced_effective_gain_histogram[
            gain_bin(output.effective_gain_permille)]++;
    }
}

static uint32_t histogram_percentile(
    const uint32_t *histogram,
    uint32_t total,
    uint32_t percentile,
    bool gain)
{
    if (histogram == NULL || total == 0u) {
        return 0u;
    }
    uint32_t target = (uint32_t)(((uint64_t)total * percentile + 99u) / 100u);
    uint32_t cumulative = 0u;
    for (size_t bin = 0u; bin < DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS; bin++) {
        cumulative += histogram[bin];
        if (cumulative >= target) {
            uint32_t upper = 1u << bin;
            return gain
                ? clamp_u64_to_u32(
                    (uint64_t)upper * DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE)
                : upper;
        }
    }
    return gain
        ? (1u << (DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS - 1u)) *
            DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE
        : 1u << (DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS - 1u);
}

denzic_audio_leveling_v1_stats_summary_t denzic_audio_leveling_v1_stats_summarize(
    const denzic_audio_leveling_v1_stats_t *stats)
{
    denzic_audio_leveling_v1_stats_summary_t summary = {0};
    if (stats == NULL) {
        return summary;
    }
    summary.frames = stats->frames;
    summary.voiced_frames = stats->voiced_frames;
    summary.gain_limited_frames = stats->gain_limited_frames;
    summary.gain_ceiling_frames = stats->gain_ceiling_frames;
    summary.limiter_frames = stats->limiter_frames;
    summary.clipped_samples = stats->clipped_samples;
    summary.raw_mean_abs_p10 = histogram_percentile(stats->raw_histogram, stats->frames, 10u, false);
    summary.raw_mean_abs_p50 = histogram_percentile(stats->raw_histogram, stats->frames, 50u, false);
    summary.raw_mean_abs_p90 = histogram_percentile(stats->raw_histogram, stats->frames, 90u, false);
    summary.post_agc_mean_abs_p10 = histogram_percentile(stats->post_agc_histogram, stats->frames, 10u, false);
    summary.post_agc_mean_abs_p50 = histogram_percentile(stats->post_agc_histogram, stats->frames, 50u, false);
    summary.post_agc_mean_abs_p90 = histogram_percentile(stats->post_agc_histogram, stats->frames, 90u, false);
    summary.effective_gain_permille_p10 = histogram_percentile(stats->effective_gain_histogram, stats->frames, 10u, true);
    summary.effective_gain_permille_p50 = histogram_percentile(stats->effective_gain_histogram, stats->frames, 50u, true);
    summary.effective_gain_permille_p90 = histogram_percentile(stats->effective_gain_histogram, stats->frames, 90u, true);
    summary.voiced_raw_mean_abs_p10 = histogram_percentile(stats->voiced_raw_histogram, stats->voiced_frames, 10u, false);
    summary.voiced_raw_mean_abs_p50 = histogram_percentile(stats->voiced_raw_histogram, stats->voiced_frames, 50u, false);
    summary.voiced_raw_mean_abs_p90 = histogram_percentile(stats->voiced_raw_histogram, stats->voiced_frames, 90u, false);
    summary.voiced_post_agc_mean_abs_p10 = histogram_percentile(stats->voiced_post_agc_histogram, stats->voiced_frames, 10u, false);
    summary.voiced_post_agc_mean_abs_p50 = histogram_percentile(stats->voiced_post_agc_histogram, stats->voiced_frames, 50u, false);
    summary.voiced_post_agc_mean_abs_p90 = histogram_percentile(stats->voiced_post_agc_histogram, stats->voiced_frames, 90u, false);
    summary.voiced_effective_gain_permille_p10 = histogram_percentile(stats->voiced_effective_gain_histogram, stats->voiced_frames, 10u, true);
    summary.voiced_effective_gain_permille_p50 = histogram_percentile(stats->voiced_effective_gain_histogram, stats->voiced_frames, 50u, true);
    summary.voiced_effective_gain_permille_p90 = histogram_percentile(stats->voiced_effective_gain_histogram, stats->voiced_frames, 90u, true);
    return summary;
}
