use std::collections::BTreeMap;
use std::io::Write;

use hound::{WavSpec, WavWriter};

use dx7::PatchBank;
use std::time::Duration;

use dx7::Patch;

/// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
/// A4 at 437hz.
pub fn generate_wav(
    patch: Patch,
    midi_notes: &[u8],
    sample_rate: u32,
    duration: Duration,
) -> Vec<u8> {
    // map from midi notes to associated buf
    let mut bufs: BTreeMap<u8, Vec<f32>> = BTreeMap::new();

    for midi_note in midi_notes {
        let mut buf = patch.generate_samples(*midi_note as f32, sample_rate, duration);

        // Find peak amplitude for normalization
        let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Normalize to -1.0 to 1.0 range if needed, with headroom
        let normalize_factor = if peak > 0.8 { 0.8 / peak } else { 1.0 };

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

    let mut ret = vec![];
    let mut cursor = std::io::Cursor::new(&mut ret);

    let mut wav_writer = WavWriter::new(&mut cursor, wav_spec).unwrap();

    for (_pitch, buf) in &bufs {
        for sample in buf {
            wav_writer.write_sample(*sample).unwrap();
        }
    }

    wav_writer.finalize().unwrap();

    ret
}

fn main() {
    const SAMPLE_RATE: u32 = 44100;

    let patch_bank_bytes =
        std::fs::read("star1-fast-decay.syx").expect("test file star1-fast-decay.syx not found");

    let patch_bank = PatchBank::new(&patch_bank_bytes);

    let patch_number = 0;
    let patch = patch_bank.patches[patch_number];

    let pitches = [60, 80, 90];

    let wav_data = generate_wav(
        patch,
        &pitches,
        SAMPLE_RATE,
        std::time::Duration::from_secs(2),
    );

    let file_name = format!("smoke-{}.wav", patch.name.iter().collect::<String>().trim());
    let mut file = std::fs::File::create(file_name).unwrap();
    file.write_all(&wav_data).unwrap();
    file.sync_all().unwrap();
}
