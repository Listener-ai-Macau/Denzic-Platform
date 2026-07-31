use denzic_audio_v1_core::leveling_v1::{Config, FrameObservation, Input, PairedLevelStats, State};

#[derive(Clone, Copy)]
struct Fixture {
    name: &'static str,
    noise_raw: u32,
    noise_post: u32,
    speech_raw: u32,
    speech_post: u32,
    speech_frames: u32,
}

fn main() {
    let fixtures = [
        Fixture {
            name: "noise",
            noise_raw: 16,
            noise_post: 400,
            speech_raw: 16,
            speech_post: 400,
            speech_frames: 0,
        },
        Fixture {
            name: "far",
            noise_raw: 8,
            noise_post: 200,
            speech_raw: 12,
            speech_post: 360,
            speech_frames: 100,
        },
        Fixture {
            name: "soft",
            noise_raw: 8,
            noise_post: 200,
            speech_raw: 20,
            speech_post: 400,
            speech_frames: 100,
        },
        Fixture {
            name: "normal",
            noise_raw: 8,
            noise_post: 160,
            speech_raw: 80,
            speech_post: 720,
            speech_frames: 100,
        },
        Fixture {
            name: "loud",
            noise_raw: 8,
            noise_post: 160,
            speech_raw: 800,
            speech_post: 4_000,
            speech_frames: 100,
        },
    ];
    println!("schema=denzic_audio_leveling_fixture_report_v1");
    println!("profile,frames,voiced,voiced_raw_p50,voiced_post_p50,voiced_gain_p50,last_scale,last_noise_floor,last_allowed_gain,gain_limited,gain_ceiling,limiter,clipped");
    for fixture in fixtures {
        report(fixture);
    }
}

fn report(fixture: Fixture) {
    let config = Config::default();
    let mut state = State::new(config);
    let mut stats = PairedLevelStats::default();
    let mut last = state.step(
        config,
        Input {
            raw_mean_abs: fixture.noise_raw,
            post_agc_mean_abs: fixture.noise_post,
            elapsed_ms: 10,
            speech_detected: false,
        },
    );
    for _ in 0..500 {
        let input = Input {
            raw_mean_abs: fixture.noise_raw,
            post_agc_mean_abs: fixture.noise_post,
            elapsed_ms: 10,
            speech_detected: false,
        };
        last = state.step(config, input);
        stats.observe(FrameObservation {
            input,
            output: last,
            limiter_applied: false,
            clipped_samples: 0,
        });
    }
    for _ in 0..fixture.speech_frames {
        let input = Input {
            raw_mean_abs: fixture.speech_raw,
            post_agc_mean_abs: fixture.speech_post,
            elapsed_ms: 10,
            speech_detected: true,
        };
        last = state.step(config, input);
        stats.observe(FrameObservation {
            input,
            output: last,
            limiter_applied: false,
            clipped_samples: 0,
        });
    }
    let summary = stats.summarize();
    println!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}",
        fixture.name,
        summary.frames,
        summary.voiced_frames,
        summary.voiced_raw_mean_abs_p50,
        summary.voiced_post_agc_mean_abs_p50,
        summary.voiced_effective_gain_permille_p50,
        last.scale_permille,
        last.noise_floor_mean_abs,
        last.allowed_gain_permille,
        summary.gain_limited_frames,
        summary.gain_ceiling_frames,
        summary.limiter_frames,
        summary.clipped_samples,
    );
}
