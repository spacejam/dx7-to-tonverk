
//! Microtuning support for non-equal temperament scales
//!
//! Supports SCL (scale) and KBM (keyboard mapping) files for alternative
//! tuning systems and microtonal music.

use serde::{Deserialize, Serialize};

/// Tuning state for microtonal support
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TuningState {
    /// Whether microtuning is enabled
    pub enabled: bool,

    /// Scale data (cents deviations from equal temperament)
    pub scale: Vec<f64>,

    /// Root note for the scale
    pub root_note: u8,

    /// Reference frequency (usually A4 = 440 Hz)
    pub reference_freq: f64,

    /// Reference MIDI note (usually 69 for A4)
    pub reference_note: u8,
}

impl Default for TuningState {
    fn default() -> Self {
        Self::equal_temperament()
    }
}

impl TuningState {
    /// Create standard 12-tone equal temperament tuning
    pub fn equal_temperament() -> Self {
        Self {
            enabled: false,
            scale: (0..12).map(|i| i as f64 * 100.0).collect(), // 100 cents per semitone
            root_note: 60, // C4
            reference_freq: 440.0,
            reference_note: 69, // A4
        }
    }

    /// Load tuning from SCL format data
    pub fn from_scl_data(_scl_data: &str) -> Result<Self, String> {
        // TODO: Implement SCL parser
        Err("SCL parsing not yet implemented".to_string())
    }

    /// Apply keyboard mapping from KBM format data
    pub fn apply_kbm_mapping(&mut self, _kbm_data: &str) -> Result<(), String> {
        // TODO: Implement KBM parser
        Err("KBM parsing not yet implemented".to_string())
    }

    /// Get frequency for a MIDI note number
    pub fn get_frequency(&self, midi_note: u8) -> f64 {
        if !self.enabled {
            // Standard equal temperament
            return 440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0);
        }

        // TODO: Implement microtonal frequency calculation
        // For now, fall back to equal temperament
        440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0)
    }

    /// Get cents deviation from equal temperament for a MIDI note
    pub fn get_cents_deviation(&self, midi_note: u8) -> f64 {
        if !self.enabled || self.scale.is_empty() {
            return 0.0;
        }

        let scale_degree = (midi_note as usize) % self.scale.len();
        let equal_temp_cents = (midi_note as f64 - self.root_note as f64) * 100.0;
        self.scale[scale_degree] - equal_temp_cents
    }

    /// Enable microtuning
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable microtuning (revert to equal temperament)
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set reference frequency (usually A4)
    pub fn set_reference_freq(&mut self, freq: f64) {
        self.reference_freq = freq.max(1.0);
    }

    /// Set reference MIDI note
    pub fn set_reference_note(&mut self, note: u8) {
        self.reference_note = note.min(127);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_temperament() {
        let tuning = TuningState::equal_temperament();
        assert!(!tuning.enabled);
        assert_eq!(tuning.scale.len(), 12);
        assert_eq!(tuning.reference_note, 69);
        assert_eq!(tuning.reference_freq, 440.0);
    }

    #[test]
    fn test_frequency_calculation() {
        let tuning = TuningState::equal_temperament();

        // A4 should be 440 Hz
        let freq = tuning.get_frequency(69);
        assert!((freq - 440.0).abs() < 0.001);

        // A5 should be 880 Hz
        let freq = tuning.get_frequency(81);
        assert!((freq - 880.0).abs() < 0.001);

        // A3 should be 220 Hz
        let freq = tuning.get_frequency(57);
        assert!((freq - 220.0).abs() < 0.001);
    }

    #[test]
    fn test_cents_deviation() {
        let tuning = TuningState::equal_temperament();

        // Equal temperament should have 0 deviation
        let deviation = tuning.get_cents_deviation(69);
        assert_eq!(deviation, 0.0);
    }

    #[test]
    fn test_enable_disable() {
        let mut tuning = TuningState::equal_temperament();
        assert!(!tuning.enabled);

        tuning.enable();
        assert!(tuning.enabled);

        tuning.disable();
        assert!(!tuning.enabled);
    }

    #[test]
    fn test_reference_settings() {
        let mut tuning = TuningState::equal_temperament();

        tuning.set_reference_freq(442.0);
        assert_eq!(tuning.reference_freq, 442.0);

        tuning.set_reference_note(70);
        assert_eq!(tuning.reference_note, 70);

        // Test bounds
        tuning.set_reference_freq(-1.0);
        assert_eq!(tuning.reference_freq, 1.0); // Should clamp to minimum

        tuning.set_reference_note(200);
        assert_eq!(tuning.reference_note, 127); // Should clamp to MIDI max
    }
}