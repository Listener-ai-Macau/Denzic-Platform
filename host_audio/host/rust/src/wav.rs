//! WAV container handling for the host-audio contract format
//! (16 kHz / mono / 16-bit little-endian PCM, 44-byte RIFF header).

use std::fs::{create_dir_all, File};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

use crate::{
    PCM_BLOCK_ALIGN, PCM_BYTE_RATE, PCM_CHANNELS, PCM_SAMPLE_RATE_HZ, PCM_SAMPLE_WIDTH_BITS,
    WAV_AUDIO_FORMAT_PCM, WAV_FMT_CHUNK_BYTES, WAV_HEADER_BYTES,
};

/// Build the 44-byte RIFF/WAVE PCM header for the contract format.
/// `data_size` is the number of PCM payload bytes following the header.
pub fn wav_header(data_size: u32) -> [u8; WAV_HEADER_BYTES] {
    let mut header = [0u8; WAV_HEADER_BYTES];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&data_size.saturating_add(36).to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    header[16..20].copy_from_slice(&WAV_FMT_CHUNK_BYTES.to_le_bytes());
    header[20..22].copy_from_slice(&WAV_AUDIO_FORMAT_PCM.to_le_bytes());
    header[22..24].copy_from_slice(&PCM_CHANNELS.to_le_bytes());
    header[24..28].copy_from_slice(&PCM_SAMPLE_RATE_HZ.to_le_bytes());
    header[28..32].copy_from_slice(&PCM_BYTE_RATE.to_le_bytes());
    header[32..34].copy_from_slice(&PCM_BLOCK_ALIGN.to_le_bytes());
    header[34..36].copy_from_slice(&PCM_SAMPLE_WIDTH_BITS.to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_size.to_le_bytes());
    header
}

/// Encode contract-format PCM samples as a complete RIFF WAV file in memory.
pub fn encode_wav(samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() as u32).saturating_mul(2);
    let mut wav = Vec::with_capacity(WAV_HEADER_BYTES + data_size as usize);
    wav.extend_from_slice(&wav_header(data_size));
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// Rewrite the header of an open WAV file with the given payload size and
/// seek back to the end of the file.
pub fn write_header(file: &mut File, data_size: u32) -> io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&wav_header(data_size))?;
    file.seek(SeekFrom::End(0))?;
    Ok(())
}

/// Appending WAV writer for the contract format. Writes a placeholder header
/// on create and backfills the final RIFF/data sizes on `finalize` or drop.
pub struct WavWriter {
    file: File,
    data_bytes: u32,
}

impl WavWriter {
    pub fn create(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        write_header(&mut file, 0)?;
        Ok(Self {
            file,
            data_bytes: 0,
        })
    }

    pub fn append_samples(&mut self, samples: &[i16]) -> io::Result<()> {
        for sample in samples {
            self.file.write_all(&sample.to_le_bytes())?;
        }
        self.data_bytes = self
            .data_bytes
            .saturating_add((samples.len() as u32).saturating_mul(2));
        Ok(())
    }

    pub fn append_bytes(&mut self, pcm: &[u8]) -> io::Result<()> {
        self.file.write_all(pcm)?;
        self.data_bytes = self
            .data_bytes
            .saturating_add(pcm.len().min(u32::MAX as usize) as u32);
        Ok(())
    }

    pub fn data_bytes(&self) -> u32 {
        self.data_bytes
    }

    pub fn finalize(&mut self) -> io::Result<u64> {
        write_header(&mut self.file, self.data_bytes)?;
        self.file.seek(SeekFrom::End(0))
    }
}

impl Drop for WavWriter {
    fn drop(&mut self) {
        let _ = self.finalize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_contract_layout() {
        let header = wav_header(8);
        assert_eq!(&header[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(header[4..8].try_into().unwrap()), 44);
        assert_eq!(&header[8..12], b"WAVE");
        assert_eq!(&header[12..16], b"fmt ");
        assert_eq!(u32::from_le_bytes(header[16..20].try_into().unwrap()), 16);
        assert_eq!(u16::from_le_bytes(header[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(header[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(header[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(
            u32::from_le_bytes(header[28..32].try_into().unwrap()),
            32_000
        );
        assert_eq!(u16::from_le_bytes(header[32..34].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(header[34..36].try_into().unwrap()), 16);
        assert_eq!(&header[36..40], b"data");
        assert_eq!(u32::from_le_bytes(header[40..44].try_into().unwrap()), 8);
    }

    #[test]
    fn encode_wav_produces_header_plus_le_samples() {
        let wav = encode_wav(&[1i16, i16::MAX, i16::MIN, -2i16]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 8);
        assert_eq!(
            &wav[44..],
            &[0x01, 0x00, 0xff, 0x7f, 0x00, 0x80, 0xfe, 0xff]
        );
    }

    #[test]
    fn wav_writer_backfills_header_on_drop() {
        let root = std::env::temp_dir().join("denzic-host-audio-wav-writer-test");
        let path = root.join("roundtrip.wav");
        let _ = std::fs::remove_file(&path);
        {
            let mut writer = WavWriter::create(&path).unwrap();
            writer.append_samples(&[100i16, -100, 200]).unwrap();
            writer.append_bytes(&[0x2c, 0x01]).unwrap();
        }
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), WAV_HEADER_BYTES + 8);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 44);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            PCM_SAMPLE_RATE_HZ
        );
    }
}
