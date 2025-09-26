
//! Portamento (pitch glide) implementation
//!
//! Provides smooth pitch transitions between notes when portamento is enabled.

/// Portamento processor
#[derive(Clone, Debug)]
pub struct Porta {
    current_pitch: f32,
    target_pitch: f32,
    rate: f32,
    enabled: bool,
}

impl Default for Porta {
    fn default() -> Self {
        Self::new()
    }
}

impl Porta {
    /// Create a new portamento processor
    pub fn new() -> Self {
        Self {
            current_pitch: 0.0,
            target_pitch: 0.0,
            rate: 0.0,
            enabled: false,
        }
    }

    /// Set portamento rate
    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.max(0.0);
    }

    /// Enable or disable portamento
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set target pitch (in semitones)
    pub fn set_target(&mut self, pitch: f32) {
        if !self.enabled {
            self.current_pitch = pitch;
        }
        self.target_pitch = pitch;
    }

    /// Get current pitch with portamento applied
    pub fn get_pitch(&mut self) -> f32 {
        if !self.enabled || self.rate <= 0.0 {
            return self.target_pitch;
        }

        let diff = self.target_pitch - self.current_pitch;
        if diff.abs() < 0.001 {
            self.current_pitch = self.target_pitch;
        } else {
            self.current_pitch += diff * self.rate;
        }

        self.current_pitch
    }

    /// Reset portamento state
    pub fn reset(&mut self, pitch: f32) {
        self.current_pitch = pitch;
        self.target_pitch = pitch;
    }

    /// Check if portamento is active (still gliding)
    pub fn is_active(&self) -> bool {
        self.enabled && (self.target_pitch - self.current_pitch).abs() > 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_porta_creation() {
        let porta = Porta::new();
        assert!(!porta.enabled);
        assert_eq!(porta.current_pitch, 0.0);
    }

    #[test]
    fn test_porta_disabled() {
        let mut porta = Porta::new();
        porta.set_enabled(false);
        porta.set_target(12.0);
        assert_eq!(porta.get_pitch(), 12.0);
    }

    #[test]
    fn test_porta_enabled() {
        let mut porta = Porta::new();
        porta.set_enabled(true);
        porta.set_rate(0.1);
        porta.reset(0.0);
        porta.set_target(12.0);

        // Should start moving towards target
        let initial = porta.get_pitch();
        assert!(initial > 0.0 && initial < 12.0);

        // Should continue moving
        let next = porta.get_pitch();
        assert!(next > initial && next < 12.0);
    }

    #[test]
    fn test_porta_reset() {
        let mut porta = Porta::new();
        porta.set_enabled(true);
        porta.set_target(12.0);
        porta.reset(5.0);

        assert_eq!(porta.current_pitch, 5.0);
        assert_eq!(porta.target_pitch, 5.0);
    }
}