//! Host-side Opus-16 kHz product path for Companion compress-at-record planning.
//!
//! **Product default target:** Opus mono @ 16 kHz / **16 kbps** (store ~2 KB/s).
//! Half-rate IMA remains the lightweight MCU baseline; this module is the
//! quality/compression path for host validation and future board encode.
//!
//! Implementation note: encode/decode shell out to `ffmpeg` + `libopus` when
//! present (Windows CI/dev). Pure libopus FFI is a follow-up; capacity math
//! and path contracts do not depend on FFI.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Product default: Opus 16 kbps mono 16 kHz.
pub const OPUS_PRODUCT_BITRATE_BPS: u32 = 16_000;
/// Approximate constant store rate for capacity planning (16 kbps / 8).
pub const OPUS_16KBPS_BYTES_PER_SEC: u64 = 2_000;
pub const OPUS_24KBPS_BYTES_PER_SEC: u64 = 3_000;

/// Recording region after flash replan v1 (UI 2 MiB + this 14 MiB).
pub use crate::ima_adpcm::COMPANION_RECORDING_REGION_BYTES;

/// Seconds of Opus-16 that fit the recording region.
pub fn opus16_max_seconds() -> u64 {
    crate::ima_adpcm::max_seconds_for_store_rate(
        COMPANION_RECORDING_REGION_BYTES,
        OPUS_16KBPS_BYTES_PER_SEC,
    )
}

/// True if a blob of `bytes` fits the 14 MiB recording store.
pub fn fits_recording_region(bytes: u64) -> bool {
    bytes <= COMPANION_RECORDING_REGION_BYTES
}

fn find_ffmpeg() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("FFMPEG_PATH") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Ok(pb);
        }
    }
    which("ffmpeg").or_else(|_| {
        let winget = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
            .join(r"Microsoft\WinGet\Links\ffmpeg.exe");
        if winget.is_file() {
            Ok(winget)
        } else {
            Err("ffmpeg not found (install ffmpeg with libopus, or set FFMPEG_PATH)".into())
        }
    })
}

fn which(name: &str) -> Result<PathBuf, String> {
    let output = Command::new("where")
        .arg(name)
        .output()
        .map_err(|e| format!("where {name}: {e}"))?;
    if !output.status.success() {
        return Err(format!("{name} not on PATH"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return Err(format!("{name} not on PATH"));
    }
    Ok(PathBuf::from(line))
}

/// Encode 16 kHz mono PCM WAV → Opus file at `bitrate_bps` (default 16000).
pub fn encode_wav_to_opus(wav_in: &Path, opus_out: &Path, bitrate_bps: u32) -> Result<u64, String> {
    let ff = find_ffmpeg()?;
    let kbps = (bitrate_bps.max(6000) / 1000).to_string();
    let status = Command::new(&ff)
        .args([
            "-y",
            "-i",
            wav_in.to_str().ok_or("wav path")?,
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "libopus",
            "-b:a",
            &format!("{kbps}k"),
            opus_out.to_str().ok_or("opus path")?,
        ])
        .status()
        .map_err(|e| format!("ffmpeg encode: {e}"))?;
    if !status.success() {
        return Err(format!("ffmpeg encode exit {status}"));
    }
    let meta = std::fs::metadata(opus_out).map_err(|e| e.to_string())?;
    Ok(meta.len())
}

/// Decode Opus → 16 kHz mono PCM WAV.
/// Accepts Ogg Opus (ffmpeg) or Companion COP1 packet stream (magic `COP1`).
pub fn decode_opus_to_wav(opus_in: &Path, wav_out: &Path) -> Result<u64, String> {
    let bytes = std::fs::read(opus_in).map_err(|e| format!("read opus: {e}"))?;
    if bytes.len() >= 4 && &bytes[0..4] == b"COP1" {
        return decode_cop1_to_wav_via_ffmpeg(&bytes, wav_out);
    }
    let ff = find_ffmpeg()?;
    let status = Command::new(&ff)
        .args([
            "-y",
            "-i",
            opus_in.to_str().ok_or("opus path")?,
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
            wav_out.to_str().ok_or("wav path")?,
        ])
        .status()
        .map_err(|e| format!("ffmpeg decode: {e}"))?;
    if !status.success() {
        return Err(format!("ffmpeg decode exit {status}"));
    }
    scale_and_clamp_pcm16_wav(wav_out, 1, 29_900)?;
    let meta = std::fs::metadata(wav_out).map_err(|e| e.to_string())?;
    Ok(meta.len())
}

/// COP1: magic 'COP1' + ver/ch/frame_ms/reserved + sample_rate u32 LE + [u16 len][pkt]*
fn decode_cop1_to_wav_via_ffmpeg(bytes: &[u8], wav_out: &Path) -> Result<u64, String> {
    if bytes.len() < 12 {
        return Err("COP1 too short".into());
    }
    if &bytes[0..4] != b"COP1" {
        return Err("not COP1".into());
    }
    let frame_ms = bytes[6] as u32;
    let sample_rate = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    // Live-mic uses 8 kHz; auto_fill/default may use 16 kHz. Host WAV is always 16 kHz.
    if !matches!(sample_rate, 8_000 | 12_000 | 16_000 | 24_000 | 48_000) {
        return Err(format!(
            "COP1 sample_rate={sample_rate}, expected 8000/12000/16000/24000/48000"
        ));
    }
    let frame_ms = if matches!(frame_ms, 10 | 20 | 40 | 60) {
        frame_ms
    } else {
        20
    };
    // Concatenate raw Opus packets into a temporary .opus by re-wrapping with
    // ffmpeg's libopus decode of concatenated packets is unreliable; instead
    // decode via opusdec-like path: write each packet to an ogg page using
    // ffmpeg filter is hard. Use `ffmpeg -f s16le` after host-side packet
    // decode is unavailable without FFI — so build a minimal Ogg stream via
    // `opusenc` replacement: shell to ffmpeg with raw opus in annex-b is not
    // supported. Practical path: use `ffprobe`/`ffmpeg` on a synthetic
    // concatenated file named .opus only works for Ogg.
    //
    // Fall back: decode COP1 by spawning ffmpeg for each packet is too slow.
    // Use `opus` CLI if present, else re-encode through temporary PCM by
    // calling external `opusdec` if on PATH.
    //
    // Minimal portable approach for Companion CI: use ffmpeg with
    // `-f lavfi -i anullsrc` no — instead convert COP1→Ogg with a tiny pure
    // Ogg page writer implemented below, then ffmpeg-decode.
    let ogg_tmp = wav_out.with_extension("cop1.opus");
    write_minimal_ogg_opus_from_cop1_packets(&bytes[12..], sample_rate, frame_ms, &ogg_tmp)?;
    let n = {
        let ff = find_ffmpeg()?;
        let status = Command::new(&ff)
            .args([
                "-y",
                "-i",
                ogg_tmp.to_str().ok_or("ogg path")?,
                "-ac",
                "1",
                "-ar",
                "16000",
                "-c:a",
                "pcm_s16le",
                wav_out.to_str().ok_or("wav path")?,
            ])
            .status()
            .map_err(|e| format!("ffmpeg decode COP1: {e}"))?;
        if !status.success() {
            return Err(format!("ffmpeg COP1 decode exit {status}"));
        }
        // Keep the board's proven fixed x8 preamp ahead of the constrained
        // 8 kHz SILK encoder. A physical A/B showed that moving the same total
        // gain ahead of Opus (x64 here + x12 host) destroyed ASR even though
        // drops/overruns stayed zero. x48 after decode is the bounded middle
        // ground between the owner-rejected x16 (too quiet) and x96 (too much
        // decoded noise), while 16 kHz seed/general Opus remains unity gain.
        let post_decode_gain = if sample_rate == 8_000 { 48 } else { 1 };
        scale_and_clamp_pcm16_wav(wav_out, post_decode_gain, 29_900)?;
        std::fs::metadata(wav_out).map_err(|e| e.to_string())?.len()
    };
    let _ = std::fs::remove_file(&ogg_tmp);
    Ok(n)
}

/// Bound rare Opus synthesis overshoots in the product PCM16 artifact. The
/// compressed device payload remains untouched and is retained beside the WAV.
fn scale_and_clamp_pcm16_wav(path: &Path, gain: i32, ceiling: i16) -> Result<(), String> {
    let mut wav = std::fs::read(path).map_err(|e| format!("read decoded wav: {e}"))?;
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err("decoded output is not RIFF/WAVE".into());
    }
    let mut offset = 12usize;
    let mut data_range = None;
    while offset + 8 <= wav.len() {
        let size = u32::from_le_bytes([
            wav[offset + 4],
            wav[offset + 5],
            wav[offset + 6],
            wav[offset + 7],
        ]) as usize;
        let start = offset + 8;
        let end = start.checked_add(size).ok_or("WAV chunk size overflow")?;
        if end > wav.len() {
            return Err("truncated decoded WAV chunk".into());
        }
        if &wav[offset..offset + 4] == b"data" {
            data_range = Some((start, end));
            break;
        }
        offset = end + (size & 1);
    }
    let (start, end) = data_range.ok_or("decoded WAV has no data chunk")?;
    if (end - start) % 2 != 0 {
        return Err("decoded WAV PCM16 data has odd length".into());
    }
    for sample in wav[start..end].chunks_exact_mut(2) {
        let value = i16::from_le_bytes([sample[0], sample[1]]) as i32;
        let bounded = (value * gain).clamp(-(ceiling as i32), ceiling as i32) as i16;
        sample.copy_from_slice(&bounded.to_le_bytes());
    }
    std::fs::write(path, wav).map_err(|e| format!("write bounded decoded wav: {e}"))
}

/// Very small Ogg Opus muxer for a sequence of raw Opus packets (mono).
/// Granule positions use the Opus 48 kHz clock (RFC 7845).
fn write_minimal_ogg_opus_from_cop1_packets(
    packets: &[u8],
    sample_rate: u32,
    frame_ms: u32,
    ogg_out: &Path,
) -> Result<(), String> {
    // Parse COP1 packet list.
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut i = 0usize;
    while i + 2 <= packets.len() {
        let n = u16::from_le_bytes([packets[i], packets[i + 1]]) as usize;
        i += 2;
        if n == 0 || i + n > packets.len() {
            break;
        }
        frames.push(packets[i..i + n].to_vec());
        i += n;
    }
    if frames.is_empty() {
        return Err("COP1 has no packets".into());
    }

    // Build OpusHead + OpusTags + pages with one packet each (simple, not low-overhead).
    let mut out = Vec::new();
    let serial = 0xC0_u32;
    let mut granule: u64 = 0;
    let mut page_seq: u32 = 0;
    // Opus Ogg granule is always 48 kHz samples per RFC 7845.
    let granule_per_frame = (48_000u64 * frame_ms as u64) / 1000;

    // OpusHead (19 bytes) — input sample rate is original capture rate.
    //
    // COP1 starts exactly on a packet boundary and carries no separate encoder
    // look-ahead field. Exposing the first packet with pre-skip=0 leaked the
    // SILK warm-up transient into the product WAV (all >30k samples in a real
    // 117 s board pull were confined to the first 12 ms). Skip one packet,
    // which is at most 60 ms and far below the product duration tolerance,
    // while preserving every subsequent meeting sample.
    let pre_skip_48k = ((48_000u64 * frame_ms as u64) / 1000) as u16;
    let mut head = Vec::new();
    head.extend_from_slice(b"OpusHead");
    head.push(1); // version
    head.push(1); // channels
    head.extend_from_slice(&pre_skip_48k.to_le_bytes());
    head.extend_from_slice(&sample_rate.to_le_bytes());
    head.extend_from_slice(&0i16.to_le_bytes()); // gain
    head.push(0); // mapping family
    write_ogg_page(&mut out, serial, page_seq, 0, 0x02, &[&head]); // BOS
    page_seq += 1;

    let tags = b"OpusTags\x08\x00\x00\x00Companion\x00\x00\x00\x00";
    write_ogg_page(&mut out, serial, page_seq, 0, 0x00, &[tags]);
    page_seq += 1;

    for (idx, pkt) in frames.iter().enumerate() {
        granule = granule.saturating_add(granule_per_frame);
        let eos = if idx + 1 == frames.len() {
            0x04u8
        } else {
            0x00
        };
        write_ogg_page(&mut out, serial, page_seq, granule, eos, &[pkt.as_slice()]);
        page_seq = page_seq.wrapping_add(1);
    }

    std::fs::write(ogg_out, out).map_err(|e| format!("write ogg: {e}"))
}

fn write_ogg_page(
    out: &mut Vec<u8>,
    serial: u32,
    page_seq: u32,
    granule: u64,
    header_type: u8,
    packets: &[&[u8]],
) {
    let mut segs = Vec::new();
    let mut body = Vec::new();
    for p in packets {
        let mut rem = p.len();
        let mut off = 0;
        while rem >= 255 {
            segs.push(255u8);
            body.extend_from_slice(&p[off..off + 255]);
            off += 255;
            rem -= 255;
        }
        segs.push(rem as u8);
        body.extend_from_slice(&p[off..]);
    }
    let mut page = Vec::new();
    page.extend_from_slice(b"OggS");
    page.push(0); // version
    page.push(header_type);
    page.extend_from_slice(&granule.to_le_bytes());
    page.extend_from_slice(&serial.to_le_bytes());
    page.extend_from_slice(&page_seq.to_le_bytes());
    page.extend_from_slice(&0u32.to_le_bytes()); // checksum placeholder
    page.push(segs.len() as u8);
    page.extend_from_slice(&segs);
    page.extend_from_slice(&body);
    let crc = ogg_crc32(&page);
    page[22..26].copy_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&page);
}

fn ogg_crc32(data: &[u8]) -> u32 {
    // Ogg CRC polynomial 0x04c11db7 reflected implementation.
    let mut crc: u32 = 0;
    for &b in data {
        crc ^= (b as u32) << 24;
        for _ in 0..8 {
            if crc & 0x8000_0000 != 0 {
                crc = (crc << 1) ^ 0x04c1_1db7;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Full product path: WAV → Opus-16 → WAV, return (opus_bytes, decoded_wav_bytes).
pub fn round_trip_opus16(wav_in: &Path, work_dir: &Path) -> Result<(u64, u64), String> {
    std::fs::create_dir_all(work_dir).map_err(|e| e.to_string())?;
    let opus = work_dir.join("product-opus16.opus");
    let wav_out = work_dir.join("product-opus16-decoded.wav");
    let ob = encode_wav_to_opus(wav_in, &opus, OPUS_PRODUCT_BITRATE_BPS)?;
    let wb = decode_opus_to_wav(&opus, &wav_out)?;
    if !fits_recording_region(ob) {
        return Err(format!(
            "opus blob {ob} exceeds recording region {COMPANION_RECORDING_REGION_BYTES}"
        ));
    }
    Ok((ob, wb))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm16_product_gain_scales_body_and_bounds_transients() {
        let dir = std::env::temp_dir().join("companion-pcm16-product-gain-test");
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("gain.wav");
        let samples = [100i16, -200, 3_000, -3_000];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36u32 + samples.len() as u32 * 2).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&16_000u32.to_le_bytes());
        bytes.extend_from_slice(&32_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
        for sample in samples { bytes.extend_from_slice(&sample.to_le_bytes()); }
        std::fs::write(&wav, bytes).unwrap();
        scale_and_clamp_pcm16_wav(&wav, 48, 29_900).unwrap();
        let output = std::fs::read(&wav).unwrap();
        let actual: Vec<i16> = output[44..]
            .chunks_exact(2)
            .map(|value| i16::from_le_bytes([value[0], value[1]]))
            .collect();
        assert_eq!(actual, vec![4_800, -9_600, 29_900, -29_900]);
    }

    #[test]
    fn opus16_capacity_on_14mib_region() {
        let secs = opus16_max_seconds();
        assert!(
            secs >= 120 * 60,
            "14 MiB @ 2 KB/s should hold >= 2h, got {secs}s"
        );
        assert!(fits_recording_region(OPUS_16KBPS_BYTES_PER_SEC * 600));
        // 10 min store ~1.2 MiB
        let ten = OPUS_16KBPS_BYTES_PER_SEC * 600;
        assert!(ten < 2 * 1024 * 1024);
    }

    #[test]
    fn cop1_ogg_skips_one_encoder_warmup_packet() {
        let dir = std::env::temp_dir().join("companion-cop1-preskip-test");
        std::fs::create_dir_all(&dir).unwrap();
        let ogg = dir.join("preskip.opus");
        // One length-prefixed dummy packet is enough to inspect the muxed head.
        write_minimal_ogg_opus_from_cop1_packets(&[1, 0, 0x08], 8_000, 20, &ogg)
            .expect("write Ogg");
        let bytes = std::fs::read(&ogg).unwrap();
        let pos = bytes
            .windows(8)
            .position(|w| w == b"OpusHead")
            .expect("OpusHead");
        let pre_skip = u16::from_le_bytes([bytes[pos + 10], bytes[pos + 11]]);
        assert_eq!(pre_skip, 960, "20 ms must be expressed on the 48 kHz clock");
    }

    /// Requires ffmpeg+libopus on PATH. Ignored by default in bare CI.
    #[test]
    #[ignore = "requires ffmpeg with libopus"]
    fn opus16_round_trip_short_tone() {
        let dir = std::env::temp_dir().join("companion-opus16-rt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // 1s silence-ish tone WAV 16k mono
        let wav = dir.join("tone.wav");
        let n = 16000u32;
        let data_len = n * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&16000u32.to_le_bytes());
        bytes.extend_from_slice(&32000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for i in 0..n {
            let t = i as f32 / 16000.0;
            let s = ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 8000.0) as i16;
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(&wav, &bytes).unwrap();
        let (ob, wb) = round_trip_opus16(&wav, &dir).expect("round trip");
        assert!(ob > 50 && ob < 50_000, "opus bytes={ob}");
        assert!(wb > 1000, "decoded wav bytes={wb}");
        eprintln!("opus16_round_trip opus_bytes={ob} wav_bytes={wb}");
    }
}

#[cfg(test)]
mod cop1_board_pull_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cop1_board_fixture_decodes_to_nonempty_16k_wav() {
        // Prefer validation artifact from goal run; skip if missing (CI without hardware).
        let candidates = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../../docs/validation/companion-type-board-opus-pull-minutes/board-opus-id3.cop1"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../../docs/validation/companion-type-board-opus-pull-minutes/board-opus-id3.cop1"),
        ];
        let mut cop1 = None;
        for c in candidates {
            if c.is_file() {
                cop1 = Some(c);
                break;
            }
        }
        let Some(cop1) = cop1 else {
            eprintln!("skip: board COP1 fixture not present");
            return;
        };
        let bytes = std::fs::read(&cop1).expect("read cop1");
        assert!(bytes.len() >= 12, "cop1 too short");
        assert_eq!(&bytes[0..4], b"COP1");
        let dir = std::env::temp_dir().join("denzic-cop1-test");
        let _ = std::fs::create_dir_all(&dir);
        let wav = dir.join("out.wav");
        let n = decode_opus_to_wav(&cop1, &wav).expect("decode COP1");
        assert!(n > 44, "wav bytes={n}");
        let meta = std::fs::metadata(&wav).expect("wav meta");
        assert!(meta.len() > 44);
    }
}
