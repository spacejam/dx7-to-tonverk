use std::time::Duration;

use hound::{WavSpec, WavWriter};

use dx7::Patch;

/// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
/// A4 at 437hz.
pub fn generate_wav(patch: Patch, midi_note: f32, sample_rate: u32, duration: Duration) -> Vec<u8> {
    let buf = patch.generate_samples(midi_note, sample_rate, duration);

    // Find peak amplitude for normalization
    let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // Normalize to -1.0 to 1.0 range if needed, with headroom
    let normalize_factor = if peak > 0.8 { 0.8 / peak } else { 1.0 };

    let wav_spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut ret = vec![];
    let mut cursor = std::io::Cursor::new(&mut ret);

    let mut wav_writer = WavWriter::new(&mut cursor, wav_spec).unwrap();

    for sample in &buf {
        wav_writer.write_sample(sample * normalize_factor).unwrap();
    }

    wav_writer.finalize().unwrap();

    ret
}
