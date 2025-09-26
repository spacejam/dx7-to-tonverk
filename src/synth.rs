
use crate::sysex::Dx7Patch;
use crate::fm::{FmCore, FreqLut, N};
use anyhow::{anyhow, Result};
use log::{debug, trace};

/// DX7 synthesizer for test vector generation
pub struct Dx7Synth {
    /// FM synthesis core
    fm_core: FmCore,

    /// Current patch loaded
    current_patch: Option<Dx7Patch>,

    /// Sample rate
    sample_rate: f64,

    /// Maximum note length in samples (safety limit)
    max_length_samples: usize,
}

impl Dx7Synth {
    /// Create a new DX7 synthesizer
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `max_length_seconds` - Maximum note length in seconds (safety limit)
    pub fn new(sample_rate: f64, max_length_seconds: f64) -> Self {
        // Initialize frequency lookup table
        FreqLut::init(sample_rate);

        let mut fm_core = FmCore::new(1); // Monophonic for test vectors
        fm_core.init_sample_rate(sample_rate);

        Self {
            fm_core,
            current_patch: None,
            sample_rate,
            max_length_samples: (sample_rate * max_length_seconds) as usize,
        }
    }

    /// Load a DX7 patch
    pub fn load_patch(&mut self, patch: Dx7Patch) -> Result<()> {
        // Apply patch parameters to the synthesis engine
        self.apply_patch_to_core(&patch)?;
        self.current_patch = Some(patch);
        Ok(())
    }

    /// Generate a note and return audio samples
    ///
    /// # Arguments
    /// * `midi_note` - MIDI note number (0-127)
    /// * `velocity` - MIDI velocity (0-127)
    /// * `note_length_seconds` - Maximum note length in seconds
    ///
    /// # Returns
    /// Vector of audio samples (mono, f32)
    pub fn render_note(&mut self, midi_note: u8, velocity: u8, note_length_seconds: f64) -> Result<Vec<f32>> {
        if self.current_patch.is_none() {
            return Err(anyhow!("No patch loaded"));
        }

        if midi_note > 127 {
            return Err(anyhow!("Invalid MIDI note: {}", midi_note));
        }

        if velocity > 127 {
            return Err(anyhow!("Invalid velocity: {}", velocity));
        }

        // Calculate maximum samples to generate
        let max_samples = ((note_length_seconds * self.sample_rate) as usize)
            .min(self.max_length_samples);

        let mut output_samples = Vec::with_capacity(max_samples);
        let mut audio_block = [0i32; N];
        let mut f32_block = [0.0f32; N];

        // Trigger the note
        self.fm_core.note_on(midi_note, velocity, 0);

        // Generate audio in blocks
        let mut samples_generated = 0;
        while samples_generated < max_samples {
            // Process a block of audio
            self.fm_core.process(&mut audio_block);


            // Convert i32 samples to f32
            for (i, &sample) in audio_block.iter().enumerate() {
                if samples_generated + i >= max_samples {
                    break;
                }
                let f32_sample = sample as f32 / (1i32 << 23) as f32;
                f32_block[i] = f32_sample;

                // Debug: show first few samples for debugging
                if samples_generated + i < 8 {
                    log::debug!("RENDER: Sample {}: i32={}, f32={}", samples_generated + i, sample, f32_sample);
                }
            }

            let block_size = (max_samples - samples_generated).min(N);
            output_samples.extend_from_slice(&f32_block[..block_size]);
            samples_generated += block_size;
        }

        // Release the note to ensure proper envelope release
        self.fm_core.note_off(midi_note, 0);

        // Continue generating until natural decay (if there's still room)
        let mut silence_count = 0;
        let silence_threshold = (self.sample_rate * 0.01) as usize; // 10ms of silence

        while samples_generated < max_samples {
            self.fm_core.process(&mut audio_block);

            let mut block_has_audio = false;
            for (i, &sample) in audio_block.iter().enumerate() {
                if samples_generated + i >= max_samples {
                    break;
                }

                let f32_sample = sample as f32 / (1i32 << 23) as f32;
                f32_block[i] = f32_sample;

                // Check for silence
                if f32_sample.abs() > 1e-6 {
                    block_has_audio = true;
                    silence_count = 0;
                } else {
                    silence_count += 1;
                }
            }

            let block_size = (max_samples - samples_generated).min(N);
            output_samples.extend_from_slice(&f32_block[..block_size]);
            samples_generated += block_size;

            // Stop if we have enough silence
            if !block_has_audio && silence_count > silence_threshold {
                break;
            }
        }

        // Assert that we generated at least the minimum expected number of samples
        // The minimum should be the note length duration, allowing for early termination due to silence
        let min_expected_samples = (note_length_seconds * self.sample_rate) as usize;

        assert!(
            output_samples.len() >= min_expected_samples.min(max_samples),
            "render_note failed to generate expected number of samples: got {}, expected at least {} (for {:.3}s at {:.1}Hz)",
            output_samples.len(),
            min_expected_samples.min(max_samples),
            note_length_seconds,
            self.sample_rate
        );

        // Assert that we don't return all zero samples (indicates audio pipeline failure)
        let non_zero_samples = output_samples.iter().filter(|&&x| x.abs() > 1e-8).count();
        assert!(
            non_zero_samples > 0,
            "render_note returned all zero samples ({} samples total) - audio pipeline failure. MIDI note: {}, velocity: {}, duration: {:.3}s",
            output_samples.len(),
            midi_note,
            velocity,
            note_length_seconds
        );

        Ok(output_samples)
    }

    /// Apply patch parameters to the FM core
    fn apply_patch_to_core(&mut self, patch: &Dx7Patch) -> Result<()> {
        let global = patch.get_global();

        // Debug: Print patch data info
        let patch_data = patch.to_data();
        debug!("SYNTH: Loading patch '{}', data length: {}", patch.name, patch_data.len());
        trace!("SYNTH: First 20 bytes: {:?}", &patch_data[..20.min(patch_data.len())]);
        debug!("SYNTH: Algorithm: {}", patch.global.algorithm);

        // Set up LFO parameters
        let lfo_params = [
            global.lfo_speed,
            global.lfo_delay,
            global.lfo_pitch_mod_depth,
            global.lfo_amp_mod_depth,
            global.lfo_sync,
            global.lfo_waveform,
        ];
        self.fm_core.set_lfo_params(&lfo_params);

        // Apply patch data to the FM core
        self.fm_core.load_patch(&patch.to_data());

        // Reset controllers to default state
        self.fm_core.reset_controllers();

        Ok(())
    }

    /// Get the current patch name
    pub fn current_patch_name(&self) -> Option<&str> {
        self.current_patch.as_ref().map(|p| p.name.as_str())
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Reset the synthesizer
    pub fn reset(&mut self) {
        self.fm_core.all_notes_off();
        self.fm_core.reset_controllers();
    }

    /// Get the number of active voices
    pub fn active_voices(&self) -> usize {
        self.fm_core.get_active_voice_count()
    }
}

/// Convert MIDI note number to frequency in Hz
pub fn midi_note_to_frequency(midi_note: u8) -> f64 {
    440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0)
}

/// Convert frequency to MIDI note number (approximate)
pub fn frequency_to_midi_note(frequency: f64) -> u8 {
    let note = 69.0 + 12.0 * (frequency / 440.0).log2();
    note.round().clamp(0.0, 127.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sysex::Dx7Patch;

    #[test]
    fn test_synth_creation() {
        let synth = Dx7Synth::new(44100.0, 10.0);
        assert_eq!(synth.sample_rate(), 44100.0);
        assert_eq!(synth.max_length_samples, 441000);
        assert_eq!(synth.active_voices(), 0);
    }

    #[test]
    fn test_midi_note_frequency_conversion() {
        // A4 (440 Hz) is MIDI note 69
        let freq = midi_note_to_frequency(69);
        assert!((freq - 440.0).abs() < 0.001);

        // A5 (880 Hz) is MIDI note 81
        let freq = midi_note_to_frequency(81);
        assert!((freq - 880.0).abs() < 0.001);

        // C4 (middle C) is MIDI note 60, approximately 261.63 Hz
        let freq = midi_note_to_frequency(60);
        assert!((freq - 261.63).abs() < 0.01);

        // Reverse conversion
        let note = frequency_to_midi_note(440.0);
        assert_eq!(note, 69);

        let note = frequency_to_midi_note(880.0);
        assert_eq!(note, 81);
    }

    #[test]
    fn test_patch_loading() {
        let mut synth = Dx7Synth::new(44100.0, 1.0);

        // Create a test patch
        let mut patch_data = [0u8; 155];
        patch_data[145..155].copy_from_slice(b"TEST PATCH");
        patch_data[134] = 5; // Algorithm 6 (0-based)
        patch_data[137] = 50; // LFO speed

        let patch = Dx7Patch::from_data(&patch_data).unwrap();
        assert_eq!(patch.name, "TEST PATCH");

        // Load the patch
        synth.load_patch(patch).unwrap();
        assert_eq!(synth.current_patch_name(), Some("TEST PATCH"));
    }

    #[test]
    fn test_render_note() {
        let mut synth = Dx7Synth::new(44100.0, 0.1); // Short test

        // Create a valid test patch using structured API
        let mut patch = Dx7Patch::new("TEST PATCH");

        // Set algorithm 1 (stored as 0)
        patch.global.algorithm = 0;

        // Configure operator 0 (carrier in algorithm 1) to produce sound
        patch.operators[0].rates.attack = 50;
        patch.operators[0].rates.decay1 = 50;
        patch.operators[0].rates.decay2 = 50;
        patch.operators[0].rates.release = 30;

        patch.operators[0].levels.attack = 99;
        patch.operators[0].levels.decay1 = 90;
        patch.operators[0].levels.decay2 = 80;
        patch.operators[0].levels.release = 0;
        patch.operators[0].output_level = 80;             // Output level
        patch.operators[0].coarse_freq = 1;               // 1:1 frequency ratio
        patch.operators[0].fine_freq = 0;                 // No fine tuning
        patch.operators[0].detune = 7;                    // Center detune

        synth.load_patch(patch).unwrap();

        // Render a short note
        let samples = synth.render_note(60, 100, 0.01).unwrap(); // 10ms note

        // Should have generated some samples
        assert!(!samples.is_empty());
        assert!(samples.len() <= 441); // 10ms at 44.1kHz

        // Check that samples are in valid range
        for &sample in &samples {
            assert!(sample >= -1.0 && sample <= 1.0);
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_invalid_inputs() {
        let mut synth = Dx7Synth::new(44100.0, 1.0);

        // Test rendering without a patch
        let result = synth.render_note(60, 100, 0.1);
        assert!(result.is_err());

        // Load a patch
        let patch_data = [0u8; 155];
        let patch = Dx7Patch::from_data(&patch_data).unwrap();
        synth.load_patch(patch).unwrap();

        // Test invalid MIDI note
        let result = synth.render_note(128, 100, 0.1);
        assert!(result.is_err());

        // Test invalid velocity
        let result = synth.render_note(60, 128, 0.1);
        assert!(result.is_err());
    }
}