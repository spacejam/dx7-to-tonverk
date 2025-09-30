//! DX7TV - DX7 Test Vector CLI
//!
//! Generate WAV files from DX7 SYSEX patches using FM synthesis.
//! This is a consolidated version that includes all necessary FM synthesis
//! components without DAW plugin dependencies.

/// Initialize logging for the library
pub fn init_logging() {
    env_logger::init();
}

// FM synthesis engine modules
pub mod fm {
    //! FM synthesis engine - core DX7 synthesis implementation
    pub mod constants;
    pub mod controllers;
    pub mod dx7note;
    pub mod env;
    pub mod exp2;
    pub mod fm_core;
    pub mod fm_op_kernel;
    pub mod freqlut;
    pub mod lfo;
    pub mod pitchenv;
    pub mod porta;
    pub mod ref_freq;
    pub mod sin;
    pub mod tuning;

    // Re-export commonly used items
    pub use constants::*;
    pub use controllers::Controllers;
    pub use dx7note::Dx7Note;
    pub use env::Env;
    pub use fm_core::FmCore;
    pub use freqlut::Freqlut;
    pub use lfo::Lfo;
}

// Application modules
pub mod synth;
pub mod sysex;
pub mod wav_writer;

// Re-export main types
pub use synth::Dx7Synth;
pub use sysex::{parse_sysex_data, parse_sysex_file, Dx7Patch};
pub use wav_writer::WavOutput;

/// Simple function to render a DX7 patch to a WAV file at 48kHz 24-bit
///
/// # Arguments
/// * `patch` - The DX7 patch to render
/// * `midi_note` - MIDI note number (0-127)
/// * `length_seconds` - Minimum note length in seconds
///
/// # Returns
/// Vector of f32 audio samples (mono, normalized to -1.0 to 1.0)
///
/// The function will render for the specified length plus additional time
/// for natural decay, ensuring at least 100ms of silence at the end.
pub fn render_patch(patch: Dx7Patch, midi_note: u8, length_seconds: f64) -> anyhow::Result<Vec<f32>> {
    const SAMPLE_RATE: f64 = 48000.0;
    const SILENCE_THRESHOLD_SECONDS: f64 = 0.1; // 100ms silence requirement
    const MAX_DECAY_SECONDS: f64 = 5.0; // Maximum time to wait for natural decay

    if midi_note > 127 {
        return Err(anyhow::anyhow!("Invalid MIDI note: {}", midi_note));
    }

    // Create synthesizer with extra time for decay
    let max_length = length_seconds + MAX_DECAY_SECONDS;
    let mut synth = synth::Dx7Synth::with_patch(patch, SAMPLE_RATE, max_length)?;

    // Render the note with velocity 100 (forte)
    let mut samples = synth.render_note(midi_note, 100, length_seconds)?;

    // Continue rendering beyond the requested length until we get sufficient silence
    let silence_threshold_samples = (SAMPLE_RATE * SILENCE_THRESHOLD_SECONDS) as usize;
    let mut consecutive_silence = 0;
    let max_additional_samples = (SAMPLE_RATE * MAX_DECAY_SECONDS) as usize;
    let mut additional_samples_rendered = 0;

    while consecutive_silence < silence_threshold_samples && additional_samples_rendered < max_additional_samples {
        // Render small chunks to check for silence
        const CHUNK_SECONDS: f64 = 0.01; // 10ms chunks
        match synth.render_note(midi_note, 100, CHUNK_SECONDS) {
            Ok(chunk) => {
                let mut chunk_has_audio = false;
                for &sample in &chunk {
                    if sample.abs() > 1e-6 {
                        chunk_has_audio = true;
                        consecutive_silence = 0;
                        break;
                    }
                }

                if !chunk_has_audio {
                    consecutive_silence += chunk.len();
                }

                samples.extend_from_slice(&chunk);
                additional_samples_rendered += chunk.len();
            },
            Err(_) => break, // Stop if we can't render more
        }
    }

    Ok(samples)
}

/// Render a DX7 patch directly to a 48kHz 24-bit WAV file
///
/// # Arguments
/// * `patch` - The DX7 patch to render
/// * `midi_note` - MIDI note number (0-127)
/// * `length_seconds` - Minimum note length in seconds
/// * `output_path` - Path where the WAV file should be written
pub fn render_patch_to_wav(patch: Dx7Patch, midi_note: u8, length_seconds: f64, output_path: &str) -> anyhow::Result<()> {
    let samples = render_patch(patch, midi_note, length_seconds)?;

    // Create WAV output with 100ms silence detection threshold
    let mut wav_output = WavOutput::new(output_path, 48000, 100_000)?;
    wav_output.write_samples(&samples)?;
    wav_output.finalize()?;

    Ok(())
}
