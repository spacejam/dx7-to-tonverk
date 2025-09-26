
//! Pitch envelope generator
//!
//! The pitch envelope modulates the pitch of operators over time,
//! providing pitch sweeps and other time-varying pitch effects.

use super::env::Env;

/// Pitch envelope generator
#[derive(Clone, Debug)]
pub struct PitchEnv {
    env: Env,
}

impl Default for PitchEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl PitchEnv {
    /// Create a new pitch envelope
    pub fn new() -> Self {
        Self {
            env: Env::new(),
        }
    }

    /// Initialize pitch envelope
    pub fn init(&mut self, rates: &[i32; 4], levels: &[i32; 4]) {
        // Pitch envelopes don't use outlevel or rate scaling in the same way
        self.env.init(rates, levels, 0, 0);
    }

    /// Get the current pitch envelope value
    pub fn get_sample(&mut self) -> i32 {
        self.env.get_sample()
    }

    /// Handle key down/up events
    pub fn keydown(&mut self, down: bool) {
        self.env.keydown(down);
    }

    /// Get current envelope position
    pub fn get_position(&self) -> i32 {
        self.env.get_position()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitchenv_creation() {
        let pitchenv = PitchEnv::new();
        assert_eq!(pitchenv.get_position(), 0);
    }

    #[test]
    fn test_pitchenv_init() {
        let mut pitchenv = PitchEnv::new();
        let rates = [50, 50, 50, 50];
        let levels = [99, 50, 25, 0];
        pitchenv.init(&rates, &levels);
        // Should not panic
    }
}