
use anyhow::{anyhow, Result};
use hound::{WavSpec, WavWriter};
use std::i16;

/// WAV file writer with silence detection
pub struct WavOutput {
    writer: Option<WavWriter<std::io::BufWriter<std::fs::File>>>,
    spec: WavSpec,
    silence_samples: usize,
    silence_threshold_samples: usize,
    silence_threshold_amplitude: f32,
}

impl WavOutput {
    /// Create a new WAV output file
    ///
    /// # Arguments
    /// * `filename` - Output WAV filename
    /// * `sample_rate` - Sample rate in Hz
    /// * `silence_duration_us` - Silence threshold in microseconds
    pub fn new(filename: &str, sample_rate: u32, silence_duration_us: u32) -> Result<Self> {
        let spec = WavSpec {
            channels: 1,           // Mono output
            sample_rate,
            bits_per_sample: 16,   // 16-bit PCM
            sample_format: hound::SampleFormat::Int,
        };

        let writer = WavWriter::create(filename, spec)
            .map_err(|e| anyhow!("Failed to create WAV file '{}': {}", filename, e))?;

        // Calculate silence threshold in samples
        let silence_threshold_samples = ((silence_duration_us as u64 * sample_rate as u64) / 1_000_000) as usize;

        Ok(Self {
            writer: Some(writer),
            spec,
            silence_samples: 0,
            silence_threshold_samples,
            silence_threshold_amplitude: 1.0 / 32768.0, // Very quiet threshold
        })
    }

    /// Write audio samples to the WAV file
    ///
    /// Returns `true` if silence threshold has been exceeded, `false` otherwise
    pub fn write_samples(&mut self, samples: &[f32]) -> Result<bool> {
        let writer = self.writer.as_mut()
            .ok_or_else(|| anyhow!("WAV writer is closed"))?;

        for &sample in samples {
            // Convert float sample to 16-bit PCM
            let pcm_sample = if sample.is_finite() {
                let clamped = sample.clamp(-1.0, 1.0);
                (clamped * 32767.0) as i16
            } else {
                0
            };

            writer.write_sample(pcm_sample)
                .map_err(|e| anyhow!("Failed to write WAV sample: {}", e))?;

            // Check for silence
            if sample.abs() <= self.silence_threshold_amplitude {
                self.silence_samples += 1;
            } else {
                self.silence_samples = 0; // Reset silence counter
            }

            // Check if we've exceeded the silence threshold
            if self.silence_samples >= self.silence_threshold_samples {
                return Ok(true); // Silence threshold exceeded
            }
        }

        Ok(false) // Still have audio
    }

    /// Finalize and close the WAV file
    pub fn finalize(mut self) -> Result<()> {
        if let Some(writer) = self.writer.take() {
            writer.finalize()
                .map_err(|e| anyhow!("Failed to finalize WAV file: {}", e))?;
        }
        Ok(())
    }

    /// Get the current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.spec.sample_rate
    }

    /// Get silence threshold in samples
    pub fn silence_threshold_samples(&self) -> usize {
        self.silence_threshold_samples
    }

    /// Get current silence sample count
    pub fn current_silence_samples(&self) -> usize {
        self.silence_samples
    }

    /// Reset silence detection
    pub fn reset_silence_detection(&mut self) {
        self.silence_samples = 0;
    }
}

impl Drop for WavOutput {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            let _ = writer.finalize(); // Ignore errors in destructor
        }
    }
}

/// Utility function to convert i32 samples (Q24 format) to f32
pub fn i32_to_f32_samples(input: &[i32], output: &mut [f32]) {
    assert_eq!(input.len(), output.len());

    for (i, &sample) in input.iter().enumerate() {
        // Convert from Q24 fixed-point to float
        output[i] = sample as f32 / (1i32 << 23) as f32;
    }
}

/// Utility function to mix multiple channels down to mono
pub fn mix_to_mono(input: &[f32], channels: usize, output: &mut [f32]) {
    assert_eq!(input.len(), output.len() * channels);

    for (i, chunk) in input.chunks_exact(channels).enumerate() {
        let sum: f32 = chunk.iter().sum();
        output[i] = sum / channels as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_wav_output_creation() {
        let temp_file = "/tmp/test_output.wav";

        // Clean up any existing file
        let _ = fs::remove_file(temp_file);

        let wav_output = WavOutput::new(temp_file, 44100, 100_000).unwrap();
        assert_eq!(wav_output.sample_rate(), 44100);
        assert_eq!(wav_output.silence_threshold_samples(), 4410); // 100ms at 44.1kHz

        // Finalize to create the file
        wav_output.finalize().unwrap();

        // Check that file was created
        assert!(std::path::Path::new(temp_file).exists());

        // Clean up
        let _ = fs::remove_file(temp_file);
    }

    #[test]
    fn test_i32_to_f32_conversion() {
        let input = [1 << 23, -(1 << 23), 0, 1 << 22]; // Max, min, zero, half
        let mut output = [0.0; 4];

        i32_to_f32_samples(&input, &mut output);

        assert!((output[0] - 1.0).abs() < 0.001);      // Max positive
        assert!((output[1] - (-1.0)).abs() < 0.001);   // Max negative
        assert!((output[2] - 0.0).abs() < 0.001);      // Zero
        assert!((output[3] - 0.5).abs() < 0.001);      // Half
    }

    #[test]
    fn test_silence_detection() {
        let temp_file = "/tmp/test_silence.wav";
        let _ = fs::remove_file(temp_file);

        let mut wav_output = WavOutput::new(temp_file, 44100, 1000).unwrap(); // 1ms threshold

        // Write some audio samples
        let loud_samples = [0.5; 100];
        let silence_exceeded = wav_output.write_samples(&loud_samples).unwrap();
        assert!(!silence_exceeded); // Should not be silent yet

        // Write silence
        let silent_samples = [0.0; 100];
        let silence_exceeded = wav_output.write_samples(&silent_samples).unwrap();
        assert!(silence_exceeded); // Should detect silence now

        wav_output.finalize().unwrap();
        let _ = fs::remove_file(temp_file);
    }

    #[test]
    fn test_mix_to_mono() {
        let stereo_input = [1.0, -1.0, 0.5, -0.5, 0.0, 0.0]; // 3 stereo samples
        let mut mono_output = [0.0; 3];

        mix_to_mono(&stereo_input, 2, &mut mono_output);

        assert!((mono_output[0] - 0.0).abs() < 0.001);  // (1.0 + -1.0) / 2
        assert!((mono_output[1] - 0.0).abs() < 0.001);  // (0.5 + -0.5) / 2
        assert!((mono_output[2] - 0.0).abs() < 0.001);  // (0.0 + 0.0) / 2
    }
}