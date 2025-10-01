use std::time::Duration;

use hound::{WavSpec, WavWriter};

use dx7::fm::{
    patch::Patch,
    voice::{Parameters, Voice},
};

/// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
/// A4 at 437hz.
pub fn generate_samples(
    patch: Patch,
    midi_note: f32,
    sample_rate: u32,
    duration: Duration,
) -> Vec<f32> {
    const MAX_BLOCK_SIZE: usize = 24; // Match C++ implementation
    let n_samples = duration.as_millis() as usize * (sample_rate as usize / 1000) as usize;
    let silence_threshold = 0.0001f32;
    let silence_duration_samples = (sample_rate as usize * 100) / 1000; // 100ms

    let mut voice = Voice::new(patch, sample_rate as f32);
    let mut output = Vec::new();

    // Phase 1: Render with gate on for the requested duration
    let mut parameters = Parameters {
        gate: true,
        sustain: false,
        velocity: 1.0,
        note: midi_note,
        ..Parameters::default()
    };

    let mut remaining = n_samples;
    while remaining > 0 {
        let block_size = remaining.min(MAX_BLOCK_SIZE);
        let mut buf = vec![0.0_f32; block_size * 3]; // render_temp needs 3x size
        voice.render_temp(&parameters, &mut buf);
        output.extend_from_slice(&buf[..block_size]);
        remaining -= block_size;
    }

    // Phase 2: Turn gate off and render until 100ms of silence
    parameters.gate = false;
    let mut consecutive_silent_samples = 0;

    loop {
        let mut chunk = vec![0.0_f32; MAX_BLOCK_SIZE * 3];
        voice.render_temp(&parameters, &mut chunk);

        // Check for silence in the rendered output
        let rendered = &chunk[..MAX_BLOCK_SIZE];
        for &sample in rendered {
            if sample.abs() < silence_threshold {
                consecutive_silent_samples += 1;
            } else {
                consecutive_silent_samples = 0;
            }
        }

        output.extend_from_slice(rendered);

        // Check if we've accumulated enough silence
        if consecutive_silent_samples >= silence_duration_samples {
            // Truncate to end after the silence duration
            let truncate_to = output.len().saturating_sub(consecutive_silent_samples - silence_duration_samples);
            output.truncate(truncate_to);
            return output;
        }

        // Safety limit: don't render more than 10 seconds total
        if output.len() > sample_rate as usize * 10 {
            break;
        }
    }

    output
}

/// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
/// A4 at 437hz.
pub fn generate_wav(patch: Patch, midi_note: f32, sample_rate: u32, duration: Duration) -> Vec<u8> {
    let buf = generate_samples(patch, midi_note, sample_rate, duration);

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
