#ifndef DENZIC_VOICE_ACTIVATION_V1_H
#define DENZIC_VOICE_ACTIVATION_V1_H

#include <stdbool.h>
#include <stdint.h>

#include "denzic_voice_activation_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t speech_confirm_ms;
    uint32_t pre_roll_ms;
    uint32_t silence_stop_ms;
    uint32_t tail_ms;
    uint32_t min_session_ms;
    uint32_t max_session_ms;
    uint32_t cooldown_ms;
} denzic_voice_activation_v1_config_t;

typedef struct {
    uint32_t elapsed_ms;
    bool enabled;
    bool auto_start_enabled;
    bool auto_stop_enabled;
    bool recording_active;
    bool speech_detected;
    bool start_blocked;
} denzic_voice_activation_v1_input_t;

typedef struct {
    denzic_voice_activation_v1_state_t state;
    denzic_voice_activation_v1_action_t action;
    denzic_voice_activation_v1_stop_reason_t stop_reason;
    uint32_t pre_roll_ms;
} denzic_voice_activation_v1_decision_t;

typedef struct {
    denzic_voice_activation_v1_state_t state;
    uint32_t speech_ms;
    uint32_t silence_ms;
    uint32_t recording_ms;
    uint32_t cooldown_ms;
} denzic_voice_activation_v1_machine_t;

denzic_voice_activation_v1_config_t denzic_voice_activation_v1_default_config(void);
void denzic_voice_activation_v1_reset(denzic_voice_activation_v1_machine_t *machine);
denzic_voice_activation_v1_decision_t denzic_voice_activation_v1_step(
    denzic_voice_activation_v1_machine_t *machine,
    const denzic_voice_activation_v1_config_t *config,
    const denzic_voice_activation_v1_input_t *input);

#ifdef __cplusplus
}
#endif

#endif
