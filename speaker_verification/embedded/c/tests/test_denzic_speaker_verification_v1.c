#include "denzic_speaker_verification_v1.h"

#include <stdio.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

static denzic_speaker_verification_v1_input_t automatic_input(void)
{
    return (denzic_speaker_verification_v1_input_t) {
        .enabled = true,
        .enrolled = true,
        .origin = DENZIC_SPEAKER_VERIFICATION_V1_CANDIDATE_ORIGIN_AUTOMATIC,
        .candidate_complete = true,
        .candidate_ms = 2200u,
        .verdict = DENZIC_SPEAKER_VERIFICATION_V1_VERDICT_MATCH,
    };
}

int main(void)
{
    denzic_speaker_verification_v1_machine_t machine = {0};
    denzic_speaker_verification_v1_input_t input = automatic_input();
    denzic_speaker_verification_v1_decision_t result =
        denzic_speaker_verification_v1_step(&machine, &input);
    CHECK(result.action == DENZIC_SPEAKER_VERIFICATION_V1_ACTION_RELEASE);

    input.verdict = DENZIC_SPEAKER_VERIFICATION_V1_VERDICT_NON_MATCH;
    result = denzic_speaker_verification_v1_step(&machine, &input);
    CHECK(result.action == DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);

    input.verdict = DENZIC_SPEAKER_VERIFICATION_V1_VERDICT_UNAVAILABLE;
    result = denzic_speaker_verification_v1_step(&machine, &input);
    CHECK(result.state == DENZIC_SPEAKER_VERIFICATION_V1_STATE_ERROR);
    CHECK(result.action == DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);

    input = automatic_input();
    input.candidate_complete = false;
    result = denzic_speaker_verification_v1_step(&machine, &input);
    CHECK(result.action == DENZIC_SPEAKER_VERIFICATION_V1_ACTION_BUFFER);

    input = automatic_input();
    input.origin = DENZIC_SPEAKER_VERIFICATION_V1_CANDIDATE_ORIGIN_MANUAL;
    result = denzic_speaker_verification_v1_step(&machine, &input);
    CHECK(result.action == DENZIC_SPEAKER_VERIFICATION_V1_ACTION_BYPASS);

    input = automatic_input();
    input.enrolled = false;
    result = denzic_speaker_verification_v1_step(&machine, &input);
    CHECK(result.action == DENZIC_SPEAKER_VERIFICATION_V1_ACTION_DISCARD);
    return 0;
}
