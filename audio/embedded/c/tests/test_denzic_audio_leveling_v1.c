#include "denzic_audio_leveling_v1.h"

#include <assert.h>
#include <stdio.h>

static denzic_audio_leveling_v1_output_t step(
    denzic_audio_leveling_v1_state_t *state,
    const denzic_audio_leveling_v1_config_t *config,
    uint32_t raw,
    uint32_t post,
    bool speech)
{
    denzic_audio_leveling_v1_input_t input = {
        .raw_mean_abs = raw,
        .post_agc_mean_abs = post,
        .elapsed_ms = 10u,
        .speech_detected = speech,
    };
    return denzic_audio_leveling_v1_step(state, config, input);
}

int main(void)
{
    denzic_audio_leveling_v1_config_t config = denzic_audio_leveling_v1_default_config();
    denzic_audio_leveling_v1_state_t state;
    denzic_audio_leveling_v1_reset(&state, &config);

    denzic_audio_leveling_v1_output_t output = {0};
    for (int index = 0; index < 500; index++) {
        output = step(&state, &config, 16u, 400u, false);
    }
    assert(output.noise_floor_mean_abs >= 14u);
    assert(output.scale_permille < DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE);
    assert(output.gain_limited);

    uint32_t quiet_scale = output.scale_permille;
    output = step(&state, &config, 20u, 360u, true);
    assert(output.speech_held);
    assert(output.scale_permille > quiet_scale);
    for (int index = 0; index < 20; index++) {
        output = step(&state, &config, 20u, 360u, true);
    }
    assert(output.scale_permille == DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE);
    for (int index = 0; index < 20; index++) {
        output = step(&state, &config, 16u, 400u, false);
    }
    assert(output.speech_held);
    assert(output.scale_permille == DENZIC_AUDIO_V1_LEVELING_SCALE_ONE_PERMILLE);

    denzic_audio_leveling_v1_stats_t stats;
    denzic_audio_leveling_v1_stats_reset(&stats);
    denzic_audio_leveling_v1_input_t input = {
        .raw_mean_abs = 16u,
        .post_agc_mean_abs = 400u,
        .elapsed_ms = 10u,
        .speech_detected = true,
    };
    denzic_audio_leveling_v1_stats_observe(&stats, input, output, true, 0u);
    denzic_audio_leveling_v1_stats_summary_t summary =
        denzic_audio_leveling_v1_stats_summarize(&stats);
    assert(summary.frames == 1u);
    assert(summary.voiced_frames == 1u);
    assert(summary.limiter_frames == 1u);
    assert(summary.raw_mean_abs_p50 >= 16u);
    assert(summary.post_agc_mean_abs_p50 >= 256u);
    assert(summary.effective_gain_permille_p50 >= 16000u);

    puts("audio leveling v1 tests passed");
    return 0;
}
