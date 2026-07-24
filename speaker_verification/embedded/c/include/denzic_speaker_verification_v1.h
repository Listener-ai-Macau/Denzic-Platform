#ifndef DENZIC_SPEAKER_VERIFICATION_V1_H
#define DENZIC_SPEAKER_VERIFICATION_V1_H

#include <stdbool.h>
#include <stdint.h>

#include "denzic_speaker_verification_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    bool enabled;
    bool enrolled;
    denzic_speaker_verification_v1_candidate_origin_t origin;
    bool candidate_complete;
    uint32_t candidate_ms;
    denzic_speaker_verification_v1_verdict_t verdict;
} denzic_speaker_verification_v1_input_t;

typedef struct {
    denzic_speaker_verification_v1_state_t state;
    denzic_speaker_verification_v1_action_t action;
} denzic_speaker_verification_v1_decision_t;

typedef struct {
    denzic_speaker_verification_v1_state_t state;
} denzic_speaker_verification_v1_machine_t;

void denzic_speaker_verification_v1_reset(
    denzic_speaker_verification_v1_machine_t *machine);
denzic_speaker_verification_v1_decision_t denzic_speaker_verification_v1_step(
    denzic_speaker_verification_v1_machine_t *machine,
    const denzic_speaker_verification_v1_input_t *input);

#ifdef __cplusplus
}
#endif

#endif
