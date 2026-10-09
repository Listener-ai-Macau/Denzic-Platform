//! Statically linked, mono Opus packet decoder. Product containers, capture
//! gain, pre-skip and storage geometry belong to the product adapter.
use std::{ffi::c_void, ptr::NonNull};

extern "C" {
    fn opus_decoder_create(sample_rate:i32, channels:i32, error:*mut i32)->*mut c_void;
    fn opus_decoder_destroy(decoder:*mut c_void);
    fn opus_decode(decoder:*mut c_void, data:*const u8, len:i32, pcm:*mut i16, frame_size:i32, decode_fec:i32)->i32;
    fn opus_packet_get_nb_samples(data:*const u8, len:i32, sample_rate:i32)->i32;
    fn opus_packet_get_nb_channels(data:*const u8)->i32;
}

pub struct MonoDecoder { decoder:NonNull<c_void>, sample_rate:u32 }
impl MonoDecoder {
    pub fn new(sample_rate:u32)->Result<Self,String> {
        if !matches!(sample_rate,8000|12000|16000|24000|48000) { return Err("unsupported Opus output sample rate".into()); }
        let mut error=0;
        let decoder=unsafe { opus_decoder_create(sample_rate as i32,1,&mut error) };
        let decoder=NonNull::new(decoder).ok_or_else(||format!("Opus decoder initialization failed: {error}"))?;
        if error!=0 { unsafe {opus_decoder_destroy(decoder.as_ptr())}; return Err(format!("Opus decoder initialization failed: {error}")); }
        Ok(Self {decoder,sample_rate})
    }
    pub fn decode(&mut self, packet:&[u8])->Result<Vec<i16>,String> {
        // Opus limits a packet to 120 ms; bound input and output before FFI.
        if packet.is_empty() || packet.len()>1275*48 { return Err("invalid Opus packet length".into()); }
        let samples=unsafe {opus_packet_get_nb_samples(packet.as_ptr(),packet.len() as i32,self.sample_rate as i32)};
        if samples<=0 || samples as u32>self.sample_rate*120/1000 { return Err("invalid Opus packet duration".into()); }
        if unsafe {opus_packet_get_nb_channels(packet.as_ptr())}!=1 { return Err("expected mono Opus packet".into()); }
        let mut pcm=vec![0i16;samples as usize];
        let decoded=unsafe {opus_decode(self.decoder.as_ptr(),packet.as_ptr(),packet.len() as i32,pcm.as_mut_ptr(),samples,0)};
        if decoded!=samples { return Err(format!("Opus packet decode failed: {decoded}")); }
        Ok(pcm)
    }
}
impl Drop for MonoDecoder { fn drop(&mut self) {unsafe {opus_decoder_destroy(self.decoder.as_ptr())}} }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn real_decoder_accepts_standard_mono_silence_and_rejects_malformed_packet() {
        let mut decoder=MonoDecoder::new(16000).unwrap();
        let pcm=decoder.decode(&[0xf8,0xff,0xfe]).unwrap();
        assert_eq!(pcm.len(),320);
        assert!(decoder.decode(&[]).is_err());
        assert!(decoder.decode(&[0xff]).is_err());
        assert!(MonoDecoder::new(44100).is_err());
    }
}
