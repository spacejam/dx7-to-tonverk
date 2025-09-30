//! Reference implementation frequency calculations
//!
//! This module implements the simple, direct frequency calculation approach
//! used in the reference implementation, replacing the complex logarithmic
//! approach from Dexed.

use crate::sysex::Dx7Patch;
use log::debug;

/// Coarse frequency lookup table in semitones (from reference dx_units.cc)
const LUT_COARSE: [f32; 32] = [
    -12.000000f32,
    0.000000f32,
    12.000000f32,
    19.019550f32,
    24.000000f32,
    27.863137f32,
    31.019550f32,
    33.688259f32,
    36.000000f32,
    38.039100f32,
    39.863137f32,
    41.513180f32,
    43.019550f32,
    44.405276f32,
    45.688259f32,
    46.882687f32,
    48.000000f32,
    49.049554f32,
    50.039100f32,
    50.975130f32,
    51.863137f32,
    52.707809f32,
    53.513180f32,
    54.282743f32,
    55.019550f32,
    55.726274f32,
    56.405276f32,
    57.058650f32,
    57.688259f32,
    58.295772f32,
    58.882687f32,
    59.450356f32,
];

/// Convert semitones to frequency ratio
/// This is a simplified version of stmlib::SemitonesToRatioSafe
fn semitones_to_ratio(semitones: f32) -> f32 {
    2.0_f32.powf(semitones / 12.0)
}

/// Calculate frequency ratio for an operator (from reference dx_units.h)
pub fn frequency_ratio(mode: u8, coarse: u8, fine: u8, detune: u8) -> f32 {
    // Fine tuning multiplier (only in ratio mode with non-zero fine)
    let detune_mult = if mode == 0 && fine != 0 {
        1.0 + 0.01 * fine as f32
    } else {
        1.0
    };

    // Base frequency in semitones
    let base = if mode == 0 {
        // Ratio mode: use coarse frequency lookup table
        LUT_COARSE[coarse as usize & 31]
    } else {
        // Fixed frequency mode
        ((coarse & 3) as f32 * 100.0 + fine as f32) * 0.39864
    };

    // Add detune (-7 to +7 range, center is 7)
    let detune_semitones = (detune as f32 - 7.0) * 0.015;
    let total_semitones = base + detune_semitones;

    semitones_to_ratio(total_semitones) * detune_mult
}

/// Calculate base frequency for a MIDI note (from reference voice.h)
pub fn base_frequency(midi_note: u8, sample_rate: f64, pitch_mod: f32) -> f32 {
    let a0 = 55.0 / sample_rate as f32;
    let note_with_mod = midi_note as f32 - 9.0 + pitch_mod * 12.0;
    a0 * 0.25 * semitones_to_ratio(note_with_mod)
}

/// Calculate operator frequency as phase increment per sample (from reference voice.h)
pub fn operator_frequency(ratio: f32, base_freq: f32, one_hz: f32) -> f32 {
    if ratio < 0.0 {
        // Fixed frequency mode (ratio stores negative value)
        -ratio * one_hz
    } else {
        // Ratio mode
        ratio * base_freq
    }
}

/// Pre-compute frequency ratios for all operators in a patch
pub fn compute_frequency_ratios(patch: &Dx7Patch) -> [f32; 6] {
    let mut ratios = [0.0; 6];

    for i in 0..6 {
        let op = &patch.operators[i];
        let ratio = frequency_ratio(op.osc_mode, op.coarse_freq, op.fine_freq, op.detune);

        // Store the sign to indicate mode (positive for ratio, negative for fixed)
        ratios[i] = if op.osc_mode == 0 { ratio } else { -ratio };

        debug!("Op {}: mode={}, coarse={}, fine={}, detune={} -> ratio={}",
               i, op.osc_mode, op.coarse_freq, op.fine_freq, op.detune, ratios[i]);
    }

    ratios
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semitones_to_ratio() {
        // Test basic semitone conversions
        assert!((semitones_to_ratio(0.0) - 1.0).abs() < 0.001); // 0 semitones = 1:1 ratio
        assert!((semitones_to_ratio(12.0) - 2.0).abs() < 0.001); // 1 octave = 2:1 ratio
        assert!((semitones_to_ratio(-12.0) - 0.5).abs() < 0.001); // -1 octave = 1:2 ratio
        assert!((semitones_to_ratio(19.019550) - 3.0).abs() < 0.001); // Should be 3:1 ratio
    }

    #[test]
    fn test_frequency_ratio() {
        // Test coarse frequency ratios
        let ratio_1_1 = frequency_ratio(0, 1, 0, 7); // 1:1 ratio, no detune
        assert!((ratio_1_1 - 1.0).abs() < 0.001);

        let ratio_2_1 = frequency_ratio(0, 2, 0, 7); // 2:1 ratio
        assert!((ratio_2_1 - 2.0).abs() < 0.001);

        let ratio_3_1 = frequency_ratio(0, 3, 0, 7); // 3:1 ratio
        assert!((ratio_3_1 - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_base_frequency() {
        // Test MIDI note 69 (A4) = 440 Hz at 44100 Hz sample rate
        let base_freq = base_frequency(69, 44100.0, 0.0);

        // Expected calculation: a0 * 0.25 * semitones_to_ratio(69 - 9)
        // a0 = 55.0 / 44100.0, note_offset = 60 semitones = 5 octaves = 2^5 = 32
        let expected_a4_freq = (55.0 / 44100.0) * 0.25 * semitones_to_ratio(60.0);
        println!("A4: base_freq={}, expected={}", base_freq, expected_a4_freq);
        assert!((base_freq - expected_a4_freq).abs() < 0.001);

        // Test MIDI note 60 (C4) ≈ 261.63 Hz
        let c4_base = base_frequency(60, 44100.0, 0.0);
        let expected_c4_freq = (55.0 / 44100.0) * 0.25 * semitones_to_ratio(51.0);
        println!("C4: base_freq={}, expected={}", c4_base, expected_c4_freq);
        assert!((c4_base - expected_c4_freq).abs() < 0.01);
    }

    #[test]
    fn test_operator_frequency() {
        let sample_rate = 44100.0;
        let one_hz = 1.0 / sample_rate;
        let base_freq = base_frequency(69, sample_rate, 0.0); // A4

        // Test ratio mode (positive ratio)
        let freq_1_1 = operator_frequency(1.0, base_freq, one_hz);
        assert!((freq_1_1 - base_freq).abs() < 0.001);

        let freq_2_1 = operator_frequency(2.0, base_freq, one_hz);
        assert!((freq_2_1 - 2.0 * base_freq).abs() < 0.001);

        // Test fixed mode (negative ratio indicates fixed frequency)
        let fixed_freq = operator_frequency(-100.0, base_freq, one_hz);
        let expected_fixed = 100.0 * one_hz;
        assert!((fixed_freq - expected_fixed).abs() < 0.001);
    }
}