fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: compress_wav_half_ima <in.wav> <out.bin>");
        std::process::exit(2);
    }
    let bytes = std::fs::read(&args[1]).expect("read");
    if bytes.len() < 44 {
        panic!("short wav");
    }
    // naive: assume PCM starts at 44 (true for our ffmpeg output)
    let pcm = &bytes[44..];
    let mut samples = Vec::with_capacity(pcm.len() / 2);
    let mut i = 0;
    while i + 1 < pcm.len() {
        samples.push(i16::from_le_bytes([pcm[i], pcm[i + 1]]));
        i += 2;
    }
    let (blob, _) = denzic_host_audio_v1_core::ima_adpcm::compress_pcm16k_half_ima(&samples, 512);
    std::fs::write(&args[2], &blob).expect("write");
    let back = denzic_host_audio_v1_core::ima_adpcm::decompress_half_ima_to_pcm16k(&blob).expect("dec");
    let data_len = (back.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + back.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&32000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in back {
        out.extend_from_slice(&s.to_le_bytes());
    }
    let dec_path = format!("{}-decoded.wav", args[2].trim_end_matches(".bin"));
    let _ = std::fs::write(&dec_path, out);
    println!("half_ima_bytes={}", blob.len());
    println!(
        "ratio={:.2}",
        (samples.len() * 2) as f64 / blob.len().max(1) as f64
    );
}
