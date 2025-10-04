use std::collections::BTreeMap;

use hound::{WavSpec, WavWriter};

use std::time::Duration;

use dx7::Patch;

/// Generates the WAV file and corresponding sample start and end ranges for the subsample at each
/// pitch.
///
/// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
/// A4 at 437hz.
pub fn generate_wav(
    patch: Patch,
    midi_notes: &[u8],
    sample_rate: u32,
    duration: Duration,
) -> (Vec<u8>, Vec<(u8, usize, usize)>) {
    // map from midi notes to associated buf
    let mut bufs: BTreeMap<u8, Vec<f32>> = BTreeMap::new();

    for midi_note in midi_notes {
        let mut buf = patch.generate_samples(*midi_note as f32, sample_rate, duration);

        // Find peak amplitude for normalization
        let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Normalize to -1.0 to 1.0 range if needed, with headroom
        let normalize_factor = if peak > 0.5 { 0.5 / peak } else { 1.0 };

        for sample in &mut buf {
            *sample *= normalize_factor;
        }

        bufs.insert(*midi_note, buf);
    }

    let wav_spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    // Find the longest buffer
    let max_len = bufs.values().map(|buf| buf.len()).max().unwrap_or(0);

    let mut wav = vec![];
    let mut pitch_start_end = vec![];
    let mut cursor = std::io::Cursor::new(&mut wav);

    let mut wav_writer = WavWriter::new(&mut cursor, wav_spec).unwrap();

    let mut running_sample_count = 0;

    for (pitch, buf) in &bufs {
        // Write the actual samples
        for sample in buf {
            wav_writer.write_sample(*sample).unwrap();
        }

        // Pad with zeros to match the longest buffer
        let padding_needed = max_len - buf.len();
        for _ in 0..padding_needed {
            wav_writer.write_sample(0.0f32).unwrap();
        }

        let start = running_sample_count;
        let end = start + max_len;
        running_sample_count = end;
        pitch_start_end.push((*pitch, start, end));
    }

    wav_writer.finalize().unwrap();

    (wav, pitch_start_end)
}
