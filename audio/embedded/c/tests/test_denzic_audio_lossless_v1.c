#include "denzic_audio_lossless_v1.h"

#include <assert.h>
#include <string.h>

#define TEST_PCM_CAPACITY 1024u
#define TEST_ENCODED_CAPACITY 1024u

static size_t pcm_from_samples(const int16_t *samples, size_t sample_count, uint8_t *pcm)
{
    for (size_t i = 0; i < sample_count; ++i) {
        pcm[i * 2u] = (uint8_t)((uint16_t)samples[i] & 0xFFu);
        pcm[i * 2u + 1u] = (uint8_t)(((uint16_t)samples[i] >> 8u) & 0xFFu);
    }
    return sample_count * 2u;
}

static void assert_round_trip(const uint8_t *pcm, uint16_t pcm_bytes, uint8_t version)
{
    uint8_t encoded[TEST_ENCODED_CAPACITY];
    uint16_t encoded_bytes = 0;
    assert(denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, version, encoded, TEST_ENCODED_CAPACITY, &encoded_bytes));
    assert(encoded_bytes < pcm_bytes);

    uint8_t decoded[TEST_PCM_CAPACITY];
    size_t decoded_bytes = 0;
    assert(denzic_audio_lossless_v1_decode(
               encoded, encoded_bytes, pcm_bytes, decoded, TEST_PCM_CAPACITY,
               &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK);
    assert(decoded_bytes == pcm_bytes);
    assert(memcmp(decoded, pcm, pcm_bytes) == 0);
}

static void test_v1_fixture_decodes_exact_pcm(void)
{
    static const uint8_t compressed[] = {
        DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1, 0, 0, 0, 1, 0, 0x03,
    };
    static const int16_t expected[] = {0, 1, 2, 3};
    uint8_t expected_pcm[8];
    size_t expected_bytes = pcm_from_samples(expected, 4, expected_pcm);

    uint8_t decoded[TEST_PCM_CAPACITY];
    size_t decoded_bytes = 0;
    assert(denzic_audio_lossless_v1_decode(
               compressed, sizeof(compressed), expected_bytes, decoded,
               TEST_PCM_CAPACITY, &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK);
    assert(decoded_bytes == expected_bytes);
    assert(memcmp(decoded, expected_pcm, expected_bytes) == 0);
}

static void test_v3_fourth_order_fixture_decodes_exact_pcm(void)
{
    /* n^3 samples have a zero fourth-order residual. */
    static const uint8_t compressed[] = {
        DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3,
        DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FOURTH_ORDER << 4,
        0, 0, 1, 0, 8, 0, 27, 0,
        0x01,
    };
    static const int16_t expected[] = {0, 1, 8, 27, 64};
    uint8_t expected_pcm[10];
    size_t expected_bytes = pcm_from_samples(expected, 5, expected_pcm);

    uint8_t decoded[TEST_PCM_CAPACITY];
    size_t decoded_bytes = 0;
    assert(denzic_audio_lossless_v1_decode(
               compressed, sizeof(compressed), expected_bytes, decoded,
               TEST_PCM_CAPACITY, &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK);
    assert(decoded_bytes == expected_bytes);
    assert(memcmp(decoded, expected_pcm, expected_bytes) == 0);
}

static size_t build_speech_like_pcm(uint8_t *pcm)
{
    int16_t samples[112];
    for (int32_t index = 0; index < 112; ++index) {
        samples[index] = (int16_t)((index * 137 + (index * index * 19) % 1200) - 7000);
    }
    return pcm_from_samples(samples, 112, pcm);
}

static void test_speech_like_pcm_round_trips_without_loss(void)
{
    uint8_t pcm[TEST_PCM_CAPACITY];
    uint16_t pcm_bytes = (uint16_t)build_speech_like_pcm(pcm);
    assert_round_trip(pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1);
    assert_round_trip(pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2);
    assert_round_trip(pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3);
}

static void test_i16_extremes_decode_exact_pcm(void)
{
    /* V2, second-order predictor, k=15, seeds i16::MAX/i16::MIN; the two
     * residuals are +131070/-131070. The encoder itself refuses this frame
     * because it grows, so the wire bytes are pinned here. */
    static const uint8_t compressed[] = {
        DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2,
        (DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_SECOND_ORDER << 4) | 15,
        0xFF, 0x7F, 0x00, 0x80,
        0x80, 0xFC, 0x7F, 0xC0, 0xFD, 0x3F,
    };
    static const int16_t expected[] = {32767, -32768, 32767, -32768};
    uint8_t expected_pcm[8];
    size_t expected_bytes = pcm_from_samples(expected, 4, expected_pcm);

    uint8_t decoded[TEST_PCM_CAPACITY];
    size_t decoded_bytes = 0;
    assert(denzic_audio_lossless_v1_decode(
               compressed, sizeof(compressed), expected_bytes, decoded,
               TEST_PCM_CAPACITY, &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK);
    assert(decoded_bytes == expected_bytes);
    assert(memcmp(decoded, expected_pcm, expected_bytes) == 0);
}

static void test_encoder_refuses_frames_that_do_not_shrink(void)
{
    static const int16_t extremes[] = {32767, -32768, 32767, -32768};
    uint8_t pcm[8];
    uint16_t pcm_bytes = (uint16_t)pcm_from_samples(extremes, 4, pcm);
    uint8_t encoded[TEST_ENCODED_CAPACITY];
    uint16_t encoded_bytes = 0;
    assert(!denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2, encoded,
        TEST_ENCODED_CAPACITY, &encoded_bytes));
}

static void test_version_selection_semantics(void)
{
    uint8_t pcm[TEST_PCM_CAPACITY];
    uint16_t pcm_bytes = (uint16_t)build_speech_like_pcm(pcm);
    uint8_t encoded[TEST_ENCODED_CAPACITY];
    uint16_t encoded_bytes = 0;

    /* V1 pins the second-order predictor; the control byte carries k only. */
    assert(denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1, encoded,
        TEST_ENCODED_CAPACITY, &encoded_bytes));
    assert(encoded[0] == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1);
    assert(encoded[1] <= DENZIC_AUDIO_LOSSLESS_V1_MAX_K);

    /* V2 negotiates predictor order one or two in the control high nibble. */
    assert(denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2, encoded,
        TEST_ENCODED_CAPACITY, &encoded_bytes));
    assert(encoded[0] == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2);
    assert((encoded[1] >> 4) >= DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER);
    assert((encoded[1] >> 4) <= DENZIC_AUDIO_V1_LOSSLESS_RICE_V2_MAX_PREDICTOR);
    assert((encoded[1] & 0x0Fu) <= DENZIC_AUDIO_LOSSLESS_V1_MAX_K);

    /* V3 carries predictor-order seed samples, so the header grows with the
     * selected predictor order. */
    assert(denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3, encoded,
        TEST_ENCODED_CAPACITY, &encoded_bytes));
    assert(encoded[0] == DENZIC_AUDIO_LOSSLESS_V1_VERSION_V3);
    uint8_t v3_predictor = (uint8_t)(encoded[1] >> 4);
    assert(v3_predictor >= DENZIC_AUDIO_LOSSLESS_V1_PREDICTOR_FIRST_ORDER);
    assert(v3_predictor <= DENZIC_AUDIO_V1_LOSSLESS_RICE_V3_MAX_PREDICTOR);
    assert(encoded_bytes >= 2u + (uint16_t)v3_predictor * 2u);
    assert(memcmp(encoded + 2, pcm, (size_t)v3_predictor * 2u) == 0);

    /* Unknown versions and undersized PCM are rejected. */
    assert(!denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, 4u, encoded, TEST_ENCODED_CAPACITY, &encoded_bytes));
    assert(!denzic_audio_lossless_v1_encode(
        pcm, 4u, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1, encoded,
        TEST_ENCODED_CAPACITY, &encoded_bytes));

    /* The decoder rejects unknown versions and out-of-range V1 parameters. */
    uint8_t decoded[TEST_PCM_CAPACITY];
    size_t decoded_bytes = 0;
    static const uint8_t bad_version[] = {4u, 0, 0, 0, 1, 0, 0x03};
    assert(denzic_audio_lossless_v1_decode(
               bad_version, sizeof(bad_version), 8, decoded, TEST_PCM_CAPACITY,
               &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_UNSUPPORTED_VERSION);
    static const uint8_t bad_k[] = {
        DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1, 16u, 0, 0, 1, 0, 0x03,
    };
    assert(denzic_audio_lossless_v1_decode(
               bad_k, sizeof(bad_k), 8, decoded, TEST_PCM_CAPACITY,
               &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_INVALID_PARAMETER);
    static const uint8_t truncated[] = {DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1, 0};
    assert(denzic_audio_lossless_v1_decode(
               truncated, sizeof(truncated), 8, decoded, TEST_PCM_CAPACITY,
               &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_TRUNCATED);
}

static void test_v2_first_order_predictor_fits_a_packet_that_v1_cannot(void)
{
    int16_t samples[200];
    uint32_t state = 1;
    int16_t sample = 0;
    for (size_t i = 0; i < 200; ++i) {
        state = state * 1103515245u + 12345u;
        int16_t delta = (int16_t)((state >> 24) & 0xFFFFu) - 128;
        int32_t next = (int32_t)sample + delta;
        if (next > 32767) {
            next = 32767;
        } else if (next < -32768) {
            next = -32768;
        }
        sample = (int16_t)next;
        samples[i] = sample;
    }
    uint8_t pcm[TEST_PCM_CAPACITY];
    uint16_t pcm_bytes = (uint16_t)pcm_from_samples(samples, 200, pcm);

    uint8_t v1_encoded[TEST_ENCODED_CAPACITY];
    uint16_t v1_bytes = 0;
    assert(denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V1, v1_encoded,
        TEST_ENCODED_CAPACITY, &v1_bytes));
    uint8_t v2_encoded[TEST_ENCODED_CAPACITY];
    uint16_t v2_bytes = 0;
    assert(denzic_audio_lossless_v1_encode(
        pcm, pcm_bytes, DENZIC_AUDIO_LOSSLESS_V1_VERSION_V2, v2_encoded,
        TEST_ENCODED_CAPACITY, &v2_bytes));

    assert(v1_bytes > 224u);
    assert(v2_bytes <= 224u);

    uint8_t decoded[TEST_PCM_CAPACITY];
    size_t decoded_bytes = 0;
    assert(denzic_audio_lossless_v1_decode(
               v2_encoded, v2_bytes, pcm_bytes, decoded, TEST_PCM_CAPACITY,
               &decoded_bytes) == DENZIC_AUDIO_LOSSLESS_V1_DECODE_OK);
    assert(decoded_bytes == pcm_bytes);
    assert(memcmp(decoded, pcm, pcm_bytes) == 0);
}

int main(void)
{
    test_v1_fixture_decodes_exact_pcm();
    test_v3_fourth_order_fixture_decodes_exact_pcm();
    test_speech_like_pcm_round_trips_without_loss();
    test_i16_extremes_decode_exact_pcm();
    test_encoder_refuses_frames_that_do_not_shrink();
    test_version_selection_semantics();
    test_v2_first_order_predictor_fits_a_packet_that_v1_cannot();
    return 0;
}
