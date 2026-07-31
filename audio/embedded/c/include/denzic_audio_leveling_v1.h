#ifndef DENZIC_AUDIO_LEVELING_V1_H
#define DENZIC_AUDIO_LEVELING_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_audio_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS (16u)

typedef struct {
    uint32_t minimum_noise_floor_mean_abs;
    uint32_t initial_noise_floor_mean_abs;
    uint32_t noise_rise_time_ms;
    uint32_t noise_fall_time_ms;
    uint32_t speech_hold_ms;
    uint32_t attenuation_attack_ms;
    uint32_t attenuation_release_ms;
    uint32_t maximum_effective_gain_permille;
    uint32_t maximum_noise_output_mean_abs;
} denzic_audio_leveling_v1_config_t;

typedef struct {
    uint32_t raw_mean_abs;
    uint32_t post_agc_mean_abs;
    uint32_t elapsed_ms;
    bool speech_detected;
} denzic_audio_leveling_v1_input_t;

typedef struct {
    uint32_t scale_permille;
    uint32_t noise_floor_mean_abs;
    uint32_t effective_gain_permille;
    uint32_t allowed_gain_permille;
    uint32_t speech_hold_remaining_ms;
    bool speech_held;
    bool gain_limited;
} denzic_audio_leveling_v1_output_t;

typedef struct {
    uint32_t noise_floor_q8;
    uint32_t scale_permille;
    uint32_t speech_hold_remaining_ms;
} denzic_audio_leveling_v1_state_t;

typedef struct {
    uint32_t frames;
    uint32_t voiced_frames;
    uint32_t gain_limited_frames;
    uint32_t gain_ceiling_frames;
    uint32_t limiter_frames;
    uint64_t clipped_samples;
    uint32_t raw_histogram[DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS];
    uint32_t post_agc_histogram[DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS];
    uint32_t effective_gain_histogram[DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS];
    uint32_t voiced_raw_histogram[DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS];
    uint32_t voiced_post_agc_histogram[DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS];
    uint32_t voiced_effective_gain_histogram[DENZIC_AUDIO_LEVELING_V1_HISTOGRAM_BINS];
} denzic_audio_leveling_v1_stats_t;

typedef struct {
    uint32_t frames;
    uint32_t voiced_frames;
    uint32_t gain_limited_frames;
    uint32_t gain_ceiling_frames;
    uint32_t limiter_frames;
    uint64_t clipped_samples;
    uint32_t raw_mean_abs_p10;
    uint32_t raw_mean_abs_p50;
    uint32_t raw_mean_abs_p90;
    uint32_t post_agc_mean_abs_p10;
    uint32_t post_agc_mean_abs_p50;
    uint32_t post_agc_mean_abs_p90;
    uint32_t effective_gain_permille_p10;
    uint32_t effective_gain_permille_p50;
    uint32_t effective_gain_permille_p90;
    uint32_t voiced_raw_mean_abs_p10;
    uint32_t voiced_raw_mean_abs_p50;
    uint32_t voiced_raw_mean_abs_p90;
    uint32_t voiced_post_agc_mean_abs_p10;
    uint32_t voiced_post_agc_mean_abs_p50;
    uint32_t voiced_post_agc_mean_abs_p90;
    uint32_t voiced_effective_gain_permille_p10;
    uint32_t voiced_effective_gain_permille_p50;
    uint32_t voiced_effective_gain_permille_p90;
} denzic_audio_leveling_v1_stats_summary_t;

denzic_audio_leveling_v1_config_t denzic_audio_leveling_v1_default_config(void);

void denzic_audio_leveling_v1_reset(
    denzic_audio_leveling_v1_state_t *state,
    const denzic_audio_leveling_v1_config_t *config);

denzic_audio_leveling_v1_output_t denzic_audio_leveling_v1_step(
    denzic_audio_leveling_v1_state_t *state,
    const denzic_audio_leveling_v1_config_t *config,
    denzic_audio_leveling_v1_input_t input);

void denzic_audio_leveling_v1_stats_reset(
    denzic_audio_leveling_v1_stats_t *stats);

void denzic_audio_leveling_v1_stats_observe(
    denzic_audio_leveling_v1_stats_t *stats,
    denzic_audio_leveling_v1_input_t input,
    denzic_audio_leveling_v1_output_t output,
    bool limiter_applied,
    uint32_t clipped_samples);

denzic_audio_leveling_v1_stats_summary_t denzic_audio_leveling_v1_stats_summarize(
    const denzic_audio_leveling_v1_stats_t *stats);

#ifdef __cplusplus
}
#endif

#endif
