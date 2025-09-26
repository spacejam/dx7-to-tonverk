
//! MIDI controllers for DX7 synthesis
//!
//! This module manages MIDI controller values including pitch bend,
//! modulation wheel, breath controller, and aftertouch.

use serde::{Deserialize, Serialize};

/// MIDI controller values
///
/// Stores the current values of various MIDI controllers that affect
/// synthesis parameters. All values are stored in their native MIDI
/// ranges but can be scaled for synthesis use.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Controllers {
    /// Pitch bend value (0x0000 - 0x3FFF, center = 0x2000)
    pub pitch_bend: u16,

    /// Modulation wheel (0-127)
    pub mod_wheel: u8,

    /// Breath controller (0-127)
    pub breath: u8,

    /// Channel aftertouch (0-127)
    pub aftertouch: u8,

    /// Foot controller (0-127)
    pub foot: u8,

    /// Expression controller (0-127)
    pub expression: u8,

    /// Volume (0-127)
    pub volume: u8,
}

impl Controllers {
    /// Create new controllers with default values
    pub fn new() -> Self {
        Self {
            pitch_bend: 0x2000, // Center position
            mod_wheel: 0,
            breath: 0,
            aftertouch: 0,
            foot: 0,
            expression: 127,
            volume: 100,
        }
    }

    /// Reset all controllers to their default values
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Set pitch bend value
    ///
    /// # Arguments
    /// * `value` - 14-bit pitch bend value (0x0000-0x3FFF)
    pub fn set_pitch_bend(&mut self, value: u16) {
        self.pitch_bend = value & 0x3FFF;
    }

    /// Get pitch bend as signed value (-8192 to +8191)
    pub fn get_pitch_bend_signed(&self) -> i16 {
        (self.pitch_bend as i16) - 0x2000
    }

    /// Set modulation wheel
    pub fn set_mod_wheel(&mut self, value: u8) {
        self.mod_wheel = value & 0x7F;
    }

    /// Set breath controller
    pub fn set_breath(&mut self, value: u8) {
        self.breath = value & 0x7F;
    }

    /// Set aftertouch
    pub fn set_aftertouch(&mut self, value: u8) {
        self.aftertouch = value & 0x7F;
    }

    /// Set foot controller
    pub fn set_foot(&mut self, value: u8) {
        self.foot = value & 0x7F;
    }

    /// Set expression
    pub fn set_expression(&mut self, value: u8) {
        self.expression = value & 0x7F;
    }

    /// Set volume
    pub fn set_volume(&mut self, value: u8) {
        self.volume = value & 0x7F;
    }

    /// Get modulation amount (0.0 - 1.0)
    pub fn get_mod_amount(&self) -> f32 {
        self.mod_wheel as f32 / 127.0
    }

    /// Get breath amount (0.0 - 1.0)
    pub fn get_breath_amount(&self) -> f32 {
        self.breath as f32 / 127.0
    }

    /// Get aftertouch amount (0.0 - 1.0)
    pub fn get_aftertouch_amount(&self) -> f32 {
        self.aftertouch as f32 / 127.0
    }

    /// Get foot controller amount (0.0 - 1.0)
    pub fn get_foot_amount(&self) -> f32 {
        self.foot as f32 / 127.0
    }

    /// Get expression amount (0.0 - 1.0)
    pub fn get_expression_amount(&self) -> f32 {
        self.expression as f32 / 127.0
    }

    /// Get volume amount (0.0 - 1.0)
    pub fn get_volume_amount(&self) -> f32 {
        self.volume as f32 / 127.0
    }

    /// Get pitch bend in semitones
    ///
    /// # Arguments
    /// * `range` - Pitch bend range in semitones (typically 2.0)
    pub fn get_pitch_bend_semitones(&self, range: f32) -> f32 {
        let signed = self.get_pitch_bend_signed() as f32;
        (signed / 8192.0) * range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controllers_creation() {
        let controllers = Controllers::new();
        assert_eq!(controllers.pitch_bend, 0x2000);
        assert_eq!(controllers.mod_wheel, 0);
        assert_eq!(controllers.volume, 100);
        assert_eq!(controllers.expression, 127);
    }

    #[test]
    fn test_pitch_bend() {
        let mut controllers = Controllers::new();

        // Test setting pitch bend
        controllers.set_pitch_bend(0x3000);
        assert_eq!(controllers.pitch_bend, 0x3000);

        // Test signed conversion
        assert_eq!(controllers.get_pitch_bend_signed(), 0x1000);

        // Test center position
        controllers.set_pitch_bend(0x2000);
        assert_eq!(controllers.get_pitch_bend_signed(), 0);

        // Test negative
        controllers.set_pitch_bend(0x1000);
        assert_eq!(controllers.get_pitch_bend_signed(), -0x1000);
    }

    #[test]
    fn test_controller_amounts() {
        let mut controllers = Controllers::new();

        controllers.set_mod_wheel(64);
        assert!((controllers.get_mod_amount() - 0.504).abs() < 0.01);

        controllers.set_breath(127);
        assert_eq!(controllers.get_breath_amount(), 1.0);

        controllers.set_aftertouch(0);
        assert_eq!(controllers.get_aftertouch_amount(), 0.0);
    }

    #[test]
    fn test_pitch_bend_semitones() {
        let mut controllers = Controllers::new();

        // Test maximum up bend (+2 semitones)
        controllers.set_pitch_bend(0x3FFF);
        let semitones = controllers.get_pitch_bend_semitones(2.0);
        assert!((semitones - 2.0).abs() < 0.01);

        // Test maximum down bend (-2 semitones)
        controllers.set_pitch_bend(0x0000);
        let semitones = controllers.get_pitch_bend_semitones(2.0);
        assert!((semitones + 2.0).abs() < 0.01);

        // Test center (0 semitones)
        controllers.set_pitch_bend(0x2000);
        let semitones = controllers.get_pitch_bend_semitones(2.0);
        assert!(semitones.abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut controllers = Controllers::new();

        // Modify some values
        controllers.set_mod_wheel(100);
        controllers.set_pitch_bend(0x3000);
        controllers.set_volume(50);

        // Reset
        controllers.reset();

        // Should be back to defaults
        assert_eq!(controllers.pitch_bend, 0x2000);
        assert_eq!(controllers.mod_wheel, 0);
        assert_eq!(controllers.volume, 100);
    }

    #[test]
    fn test_value_masking() {
        let mut controllers = Controllers::new();

        // Test that values are properly masked
        controllers.set_mod_wheel(0xFF); // Should mask to 0x7F
        assert_eq!(controllers.mod_wheel, 0x7F);

        controllers.set_pitch_bend(0xFFFF); // Should mask to 0x3FFF
        assert_eq!(controllers.pitch_bend, 0x3FFF);
    }
}