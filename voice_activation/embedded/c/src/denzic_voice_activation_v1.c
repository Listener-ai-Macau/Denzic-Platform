#include "denzic_voice_activation_v1.h"

#include <limits.h>
#include <stddef.h>

static uint32_t add_saturating(uint32_t value, uint32_t increment)
{
    return increment > UINT32_MAX - value ? UINT32_MAX : value + increment;
}

static denzic_voice_activation_v1_decision_t decision_for(
    const denzic_voice_activation_v1_machine_t *machine)
{
    denzic_voice_activation_v1_decision_t decision = {
        .state = DENZIC_VOICE_ACTIVATION_V1_STATE_DISABLED,
        .action = DENZIC_VOICE_ACTIVATION_V1_ACTION_NONE,
        .stop_reason = DENZIC_VOICE_ACTIVATION_V1_STOP_REASON_NONE,
        .pre_roll_ms = 0u,
    };
    if (machine != NULL) {
        decision.state = machine->state;
    }
    return decision;
}

denzic_voice_activation_v1_config_t denzic_voice_activation_v1_default_config(void)
{
    return (denzic_voice_activation_v1_config_t) {
        .speech_confirm_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_SPEECH_CONFIRM_MS,
        .pre_roll_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_PRE_ROLL_MS,
        .silence_stop_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_SILENCE_STOP_MS,
        .tail_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_TAIL_MS,
        .min_session_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_MIN_SESSION_MS,
        .max_session_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_MAX_SESSION_MS,
        .cooldown_ms = DENZIC_VOICE_ACTIVATION_V1_DEFAULT_COOLDOWN_MS,
    };
}

void denzic_voice_activation_v1_reset(denzic_voice_activation_v1_machine_t *machine)
{
    if (machine == NULL) {
        return;
    }
    *machine = (denzic_voice_activation_v1_machine_t) {
        .state = DENZIC_VOICE_ACTIVATION_V1_STATE_DISABLED,
    };
}

denzic_voice_activation_v1_decision_t denzic_voice_activation_v1_step(
    denzic_voice_activation_v1_machine_t *machine,
    const denzic_voice_activation_v1_config_t *config,
    const denzic_voice_activation_v1_input_t *input)
{
    denzic_voice_activation_v1_decision_t decision = decision_for(machine);
    if (machine == NULL || config == NULL || input == NULL) {
        return decision;
    }
    if (!input->enabled ||
        (!input->auto_start_enabled && !input->auto_stop_enabled)) {
        denzic_voice_activation_v1_reset(machine);
        return decision_for(machine);
    }

    if (input->recording_active) {
        if (machine->state != DENZIC_VOICE_ACTIVATION_V1_STATE_RECORDING &&
            machine->state != DENZIC_VOICE_ACTIVATION_V1_STATE_TAIL) {
            machine->recording_ms = 0u;
            machine->silence_ms = 0u;
        }
        machine->recording_ms =
            add_saturating(machine->recording_ms, input->elapsed_ms);
        machine->speech_ms = 0u;
        if (input->speech_detected) {
            machine->silence_ms = 0u;
            machine->state = DENZIC_VOICE_ACTIVATION_V1_STATE_RECORDING;
        } else if (input->auto_stop_enabled) {
            machine->silence_ms =
                add_saturating(machine->silence_ms, input->elapsed_ms);
            machine->state = machine->silence_ms >= config->silence_stop_ms
                ? DENZIC_VOICE_ACTIVATION_V1_STATE_TAIL
                : DENZIC_VOICE_ACTIVATION_V1_STATE_RECORDING;
        } else {
            machine->silence_ms = 0u;
            machine->state = DENZIC_VOICE_ACTIVATION_V1_STATE_RECORDING;
        }

        decision = decision_for(machine);
        if (input->auto_stop_enabled &&
            config->max_session_ms > 0u &&
            machine->recording_ms >= config->max_session_ms) {
            decision.action = DENZIC_VOICE_ACTIVATION_V1_ACTION_STOP;
            decision.stop_reason =
                DENZIC_VOICE_ACTIVATION_V1_STOP_REASON_MAX_DURATION;
        } else if (
            input->auto_stop_enabled &&
            machine->recording_ms >= config->min_session_ms &&
            machine->silence_ms >=
                add_saturating(config->silence_stop_ms, config->tail_ms)) {
            decision.action = DENZIC_VOICE_ACTIVATION_V1_ACTION_STOP;
            decision.stop_reason =
                DENZIC_VOICE_ACTIVATION_V1_STOP_REASON_SILENCE;
        }
        return decision;
    }

    if (machine->state == DENZIC_VOICE_ACTIVATION_V1_STATE_RECORDING ||
        machine->state == DENZIC_VOICE_ACTIVATION_V1_STATE_TAIL) {
        machine->state = DENZIC_VOICE_ACTIVATION_V1_STATE_COOLDOWN;
        machine->cooldown_ms = config->cooldown_ms;
        machine->recording_ms = 0u;
        machine->silence_ms = 0u;
    }
    if (machine->state == DENZIC_VOICE_ACTIVATION_V1_STATE_COOLDOWN) {
        machine->cooldown_ms =
            input->elapsed_ms >= machine->cooldown_ms
                ? 0u
                : machine->cooldown_ms - input->elapsed_ms;
        if (machine->cooldown_ms > 0u) {
            return decision_for(machine);
        }
        machine->state = DENZIC_VOICE_ACTIVATION_V1_STATE_MONITORING;
    }

    machine->recording_ms = 0u;
    machine->silence_ms = 0u;
    if (!input->auto_start_enabled ||
        input->start_blocked ||
        !input->speech_detected) {
        machine->speech_ms = 0u;
        machine->state = DENZIC_VOICE_ACTIVATION_V1_STATE_MONITORING;
    } else {
        machine->speech_ms = add_saturating(machine->speech_ms, input->elapsed_ms);
        machine->state = DENZIC_VOICE_ACTIVATION_V1_STATE_SPEECH_CONFIRMING;
        if (machine->speech_ms >= config->speech_confirm_ms) {
            decision.action = DENZIC_VOICE_ACTIVATION_V1_ACTION_START;
            decision.pre_roll_ms = config->pre_roll_ms;
            machine->speech_ms = 0u;
        }
    }
    decision.state = machine->state;
    return decision;
}

denzic_voice_activation_v1_gate_decision_t denzic_voice_activation_v1_decide_gate(
    denzic_voice_activation_v1_phrase_signal_t phrase_signal,
    bool owner_match_known,
    bool owner_match,
    bool terminal) {
    if (phrase_signal != DENZIC_VOICE_ACTIVATION_V1_PHRASE_SIGNAL_NONE) {
        if (owner_match_known) {
            return owner_match
                ? DENZIC_VOICE_ACTIVATION_V1_GATE_DECISION_ACCEPT
                : DENZIC_VOICE_ACTIVATION_V1_GATE_DECISION_REJECT;
        }
        return terminal
            ? DENZIC_VOICE_ACTIVATION_V1_GATE_DECISION_REJECT
            : DENZIC_VOICE_ACTIVATION_V1_GATE_DECISION_PENDING;
    }
    return terminal
        ? DENZIC_VOICE_ACTIVATION_V1_GATE_DECISION_REJECT
        : DENZIC_VOICE_ACTIVATION_V1_GATE_DECISION_PENDING;
}
