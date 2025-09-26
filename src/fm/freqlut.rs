
//! Frequency lookup table for MIDI note to frequency conversion
//!
//! Converts MIDI note numbers to phase increments for oscillators.

use std::sync::Once;

const N_SAMPLES: usize = 4096;
const MAX_LOGFREQ_INT: i32 = 16;

static mut LUT: [i32; N_SAMPLES + 1] = [0; N_SAMPLES + 1];
static INIT: Once = Once::new();

/// Frequency lookup table
pub struct FreqLut;

impl FreqLut {
    /// Initialize frequency lookup table
    pub fn init(sample_rate: f64) {
        unsafe {
            INIT.call_once(|| {
                let y_start = (1u64 << (24 + MAX_LOGFREQ_INT)) as f64 / sample_rate;
                let inc = 2.0_f64.powf(1.0 / N_SAMPLES as f64);

                let mut y = y_start;
                for i in 0..=N_SAMPLES {
                    LUT[i] = (y + 0.5).floor() as i32;
                    y *= inc;
                }
            });
        }
    }

    /// Convert logarithmic frequency to phase increment
    ///
    /// # Arguments
    /// * `logfreq` - Logarithmic frequency value
    ///
    /// # Returns
    /// Phase increment value for the given frequency
    pub fn lookup_logfreq(logfreq: i32) -> i32 {
        let ix = (logfreq & 0xff_ffff) >> (24 - 12);
        let y0 = unsafe { LUT[ix as usize] };
        let y1 = unsafe { LUT[(ix + 1) as usize] };
        let dx = logfreq & ((1 << (24 - 12)) - 1);
        let scaled_dx = dx >> (24 - 12 - 8);
        y0 + (((y1 - y0) * scaled_dx) >> 8)
    }

    /// Convert MIDI note to phase increment
    ///
    /// # Arguments
    /// * `midinote` - MIDI note number (0-127)
    ///
    /// # Returns
    /// Phase increment value for the given note
    pub fn lookup(midinote: u8) -> u32 {
        // Convert MIDI note to log frequency
        // DX7 uses 12-bit fractional MIDI note representation
        let note_scaled = (midinote as i32) << 16;

        // Base frequency calculation: 440Hz at MIDI note 69 (A4)
        // Log2 frequency relative to sample rate
        let logfreq = note_scaled + (69 << 16); // Adjust for A4 = 440Hz reference

        // Use lookup table
        let phase_inc = Self::lookup_logfreq(logfreq);
        phase_inc as u32
    }

    /// Convert MIDI note with fine tuning to phase increment
    ///
    /// # Arguments
    /// * `midinote` - Base MIDI note number (0-127)
    /// * `fine_tune` - Fine tuning in cents (-100 to +100)
    ///
    /// # Returns
    /// Phase increment value for the tuned note
    pub fn lookup_fine(midinote: u8, fine_tune: i16) -> u32 {
        // Convert to 16-bit fixed point MIDI note with fine tuning
        let note_base = (midinote as i32) << 16;
        let fine_adjust = ((fine_tune as i32) << 16) / 100; // Convert cents to fractional note
        let note_scaled = note_base + fine_adjust;

        let logfreq = note_scaled + (69 << 16); // A4 reference
        let phase_inc = Self::lookup_logfreq(logfreq);
        phase_inc as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freqlut_lookup() {
        // Initialize the lookup table first
        FreqLut::init(44100.0);

        // A4 (440 Hz) is MIDI note 69
        let phase_inc = FreqLut::lookup(69);
        assert!(phase_inc > 0);

        // Higher notes should have higher phase increments
        let high_note = FreqLut::lookup(81); // A5 (880 Hz)
        assert!(high_note > phase_inc);

        // Test the fine tuning function
        let fine_tuned = FreqLut::lookup_fine(69, 50); // 50 cents sharp
        assert!(fine_tuned > phase_inc);
    }
}