#include "denzic_voice_activation_v1.h"

#include <stdio.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

static denzic_voice_activation_v1_input_t input_for(bool speech)
{
    return (denzic_voice_activation_v1_input_t) {
        .elapsed_ms = 100u,
        .enabled = true,
        .auto_start_enabled = true,
        .auto_stop_enabled = true,
        .speech_detected = speech,
    };
}

int main(void)
{
    denzic_voice_activation_v1_config_t config =
        denzic_voice_activation_v1_default_config();
    denzic_voice_activation_v1_machine_t machine = {0};
    denzic_voice_activation_v1_input_t input = input_for(false);
    denzic_voice_activation_v1_decision_t decision =
        denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_NONE);
    CHECK(decision.state == DENZIC_VOICE_ACTIVATION_V1_STATE_MONITORING);

    input.speech_detected = true;
    decision = denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_NONE);
    decision = denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_NONE);
    decision = denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_START);
    CHECK(decision.pre_roll_ms == 600u);

    input.recording_active = true;
    input.speech_detected = false;
    for (unsigned i = 0; i < 12u; ++i) {
        decision = denzic_voice_activation_v1_step(&machine, &config, &input);
        CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_NONE);
    }
    decision = denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_STOP);
    CHECK(decision.stop_reason ==
          DENZIC_VOICE_ACTIVATION_V1_STOP_REASON_SILENCE);

    input.recording_active = false;
    input.speech_detected = true;
    decision = denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.state == DENZIC_VOICE_ACTIVATION_V1_STATE_COOLDOWN);

    denzic_voice_activation_v1_reset(&machine);
    config.max_session_ms = 300u;
    input.recording_active = true;
    input.speech_detected = true;
    (void)denzic_voice_activation_v1_step(&machine, &config, &input);
    (void)denzic_voice_activation_v1_step(&machine, &config, &input);
    decision = denzic_voice_activation_v1_step(&machine, &config, &input);
    CHECK(decision.action == DENZIC_VOICE_ACTIVATION_V1_ACTION_STOP);
    CHECK(decision.stop_reason ==
          DENZIC_VOICE_ACTIVATION_V1_STOP_REASON_MAX_DURATION);
    return 0;
}
