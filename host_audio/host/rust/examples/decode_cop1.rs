fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: decode_cop1 <in.opus/cop1> <out.wav>");
        std::process::exit(2);
    }
    let n = denzic_host_audio_v1_core::decode_opus_to_wav(
        std::path::Path::new(&args[1]),
        std::path::Path::new(&args[2]),
    );
    match n {
        Ok(bytes) => {
            println!("DECODE_OK wav_bytes={bytes}");
        }
        Err(e) => {
            eprintln!("DECODE_ERR {e}");
            std::process::exit(1);
        }
    }
}
