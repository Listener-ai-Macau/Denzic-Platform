#include "denzic_speaker_verification_v1.h"

#include <stddef.h>

static denzic_speaker_verification_v1_decision_t decision(
    denzic_speaker_verification_v1_state_t state,
    denzic_speaker_verification_v1_action_t action)
{
    return (denzic_speaker_verification_v1_decision_t) {
        .state = state,
        .action = action,
    };
}

void denzic_speaker_verification_v1_reset(
    denzic_speaker_verification_v1_machine_t *machine)
{
    if (machine != NULL) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_IDLE;
    }
}

denzic_speaker_verification_v1_decision_t denzic_speaker_verification_v1_step(
    denzic_speaker_verification_v1_machine_t *machine,
    const denzic_speaker_verification_v1_input_t *input)
{
    if (machine == NULL || input == NULL) {
        return decision(
            DENZIC_SPEAKER_VERIFICATION_V1_STATE_ERROR,
            DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);
    }
    if (input->origin == DENZIC_SPEAKER_VERIFICATION_V1_CANDIDATE_ORIGIN_MANUAL ||
        !input->enabled) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_RELEASED;
        return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_BYPASS);
    }
    if (input->origin == DENZIC_SPEAKER_VERIFICATION_V1_CANDIDATE_ORIGIN_AUTOMATIC &&
        !input->enrolled) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_DISCARDED;
        return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);
    }
    if (input->candidate_ms > DENZIC_SPEAKER_VERIFICATION_V1_DEFAULT_MAX_CANDIDATE_MS) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_DISCARDED;
        return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);
    }
    if (!input->candidate_complete) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_BUFFERING;
        return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_BUFFER);
    }
    if (input->candidate_ms < DENZIC_SPEAKER_VERIFICATION_V1_DEFAULT_MIN_CANDIDATE_MS) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_DISCARDED;
        return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);
    }
    if (input->verdict == DENZIC_SPEAKER_VERIFICATION_V1_VERDICT_MATCH) {
        machine->state = DENZIC_SPEAKER_VERIFICATION_V1_STATE_RELEASED;
        return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_RELEASE);
    }
    machine->state =
        input->verdict == DENZIC_SPEAKER_VERIFICATION_V1_VERDICT_NON_MATCH
            ? DENZIC_SPEAKER_VERIFICATION_V1_STATE_DISCARDED
            : DENZIC_SPEAKER_VERIFICATION_V1_STATE_ERROR;
    return decision(machine->state, DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);
}
