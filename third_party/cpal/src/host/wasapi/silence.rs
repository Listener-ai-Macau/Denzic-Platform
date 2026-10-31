use crate::SampleFormat;

// u64 storage preserves the alignment required by every supported sample
// type. The buffer stays alive until the callback returns.
pub(super) fn silence_buffer(format: SampleFormat, len: usize) -> Vec<u64> {
    let byte_len = len * format.sample_size();
    let mut words = vec![0u64; (byte_len + 7) / 8];
    let bytes = unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, byte_len) };
    match format {
        SampleFormat::U8 => bytes.fill(128),
        SampleFormat::U16 => for s in bytes.chunks_exact_mut(2) { s.copy_from_slice(&32768u16.to_ne_bytes()); },
        SampleFormat::U32 => for s in bytes.chunks_exact_mut(4) { s.copy_from_slice(&2147483648u32.to_ne_bytes()); },
        SampleFormat::U64 => for s in bytes.chunks_exact_mut(8) { s.copy_from_slice(&9223372036854775808u64.to_ne_bytes()); },
        _ => (),
    }
    words
}

#[cfg(test)]
mod packet_tests {
    use super::*;

    #[test]
    fn silent_packet_uses_the_neutral_value_for_each_supported_format() {
        for format in [SampleFormat::I8, SampleFormat::I16, SampleFormat::I32,
            SampleFormat::I64, SampleFormat::F32, SampleFormat::F64] {
            let words = silence_buffer(format, 3);
            assert!(words.iter().all(|value| *value == 0), "{format:?}");
        }
        for (format, expected) in [(SampleFormat::U8, 128u64), (SampleFormat::U16, 32768),
            (SampleFormat::U32, 2147483648), (SampleFormat::U64, 9223372036854775808)] {
            let words = silence_buffer(format, 3);
            let bytes = unsafe { std::slice::from_raw_parts(words.as_ptr() as *const u8, 3 * format.sample_size()) };
            for sample in bytes.chunks_exact(format.sample_size()) {
                let mut value = [0u8; 8];
                value[..sample.len()].copy_from_slice(sample);
                assert_eq!(u64::from_le_bytes(value), expected, "{format:?}");
            }
        }
    }
}

