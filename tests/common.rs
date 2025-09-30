use std::time::Duration;

use hound::{WavSpec, WavWriter};

use dx7::fm::{
    patch::{Patch, PatchBank},
    voice::{Parameters, Voice},
};

pub fn generate_samples(
    patch: Patch,
    midi_note: f32,
    sample_rate: u32,
    duration: Duration,
) -> Vec<f32> {
    let n_samples = duration.as_millis() as usize * (sample_rate as usize / 1000) as usize;

    let mut buf = vec![0.0_f32; n_samples];

    let parameters = Parameters::default();

    let mut voice = Voice::new(patch, midi_note, sample_rate as f32);

    voice.render_temp(&parameters, &mut buf);

    buf
}

pub fn generate_wav(patch: Patch, midi_note: f32, sample_rate: u32, duration: Duration) -> Vec<u8> {
    let buf = generate_samples(patch, midi_note, sample_rate, duration);

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
        wav_writer.write_sample(*sample as i32).unwrap();
    }

    wav_writer.finalize().unwrap();

    ret
}
