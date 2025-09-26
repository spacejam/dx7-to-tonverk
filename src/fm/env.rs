
//! DX7 envelope generation
//!
//! This module implements the four-stage ADSR-style envelopes used in the DX7.
//! The envelope generator produces logarithmic output values that are later
//! converted to linear gain values.

use super::constants::*;
use std::sync::atomic::{AtomicU32, Ordering};

static SR_MULTIPLIER: AtomicU32 = AtomicU32::new(1 << 24);

const LEVEL_LUT: [i32; 20] = [
    0, 5, 9, 13, 17, 20, 23, 25, 27, 29, 31, 33, 35, 37, 39, 41, 42, 43, 45, 46
];

// Accurate envelope timing constants
const STATICS: [i32; 77] = [
    1764000, 1764000, 1411200, 1411200, 1190700, 1014300, 992250,
    882000, 705600, 705600, 584325, 507150, 502740, 441000, 418950,
    352800, 308700, 286650, 253575, 220500, 220500, 176400, 145530,
    145530, 125685, 110250, 110250, 88200, 88200, 74970, 61740,
    61740, 55125, 48510, 44100, 37485, 31311, 30870, 27562, 27562,
    22050, 18522, 17640, 15435, 14112, 13230, 11025, 9261, 9261, 7717,
    6615, 6615, 5512, 5512, 4410, 3969, 3969, 3439, 2866, 2690, 2249,
    1984, 1896, 1808, 1411, 1367, 1234, 1146, 926, 837, 837, 705,
    573, 573, 529, 441, 441
];

/// DX7-style envelope generator
///
/// The envelope has four stages: Attack, Decay, Sustain, and Release.
/// Each stage has configurable rate and level parameters that match
/// the original DX7 specifications.
#[derive(Clone, Debug)]
pub struct Env {
    rates: [i32; 4],
    levels: [i32; 4],
    outlevel: i32,
    rate_scaling: i32,

    // Current envelope state
    level: i32,          // Q24 format (2^24 = one doubling)
    targetlevel: i32,
    rising: bool,
    ix: i32,             // Current envelope stage (0-3 = ADSR, 4 = finished)
    inc: i32,            // Rate increment per sample
    staticcount: i32,    // Samples remaining in static phase
    down: bool,          // Key is down (true) or up (false)
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    /// Create a new envelope generator
    pub fn new() -> Self {
        Self {
            rates: [0; 4],
            levels: [0; 4],
            outlevel: 0,
            rate_scaling: 0,
            level: 0,
            targetlevel: 0,
            rising: false,
            ix: 0,
            inc: 0,
            staticcount: 0,
            down: true,
        }
    }

    /// Initialize sample rate scaling
    pub fn init_sr(sample_rate: f64) {
        let multiplier = ((44100.0 / sample_rate) * ((1u32 << 24) as f64)) as u32;
        SR_MULTIPLIER.store(multiplier, Ordering::Relaxed);
    }

    /// Initialize envelope with DX7 parameters
    ///
    /// # Arguments
    /// * `rates` - Attack, Decay, Sustain, Release rates (0-99)
    /// * `levels` - Attack, Decay, Sustain, Release levels (0-99)
    /// * `outlevel` - Output level in microsteps
    /// * `rate_scaling` - Rate scaling amount (0-63)
    pub fn init(&mut self, rates: &[i32; 4], levels: &[i32; 4], outlevel: i32, rate_scaling: i32) {
        self.rates = *rates;
        self.levels = *levels;
        self.outlevel = outlevel;
        self.rate_scaling = rate_scaling;
        self.level = 0;
        self.down = true;
        self.advance(0);
    }

    /// Update envelope parameters during playback
    pub fn update(&mut self, rates: &[i32; 4], levels: &[i32; 4], outlevel: i32, rate_scaling: i32) {
        self.rates = *rates;
        self.levels = *levels;
        self.outlevel = outlevel;
        self.rate_scaling = rate_scaling;

        if self.down {
            // Reset to sustain stage when key is down
            let newlevel = self.levels[2];
            let mut actuallevel = Self::scale_outlevel(newlevel) >> 1;
            actuallevel = (actuallevel << 6) - 4256;
            actuallevel = if actuallevel < 16 { 16 } else { actuallevel };
            self.targetlevel = actuallevel << 16;
            self.advance(2);
        }
    }

    /// Get the next envelope sample
    ///
    /// Returns the current envelope level in Q24 logarithmic format
    pub fn get_sample(&mut self) -> i32 {
        // Handle static (hold) phase
        if self.staticcount > 0 {
            self.staticcount -= N as i32;
            if self.staticcount <= 0 {
                self.staticcount = 0;
                self.advance(self.ix + 1);
            }
        }

        // Process envelope stages (Attack, Decay, Sustain, or Release if key up)
        if self.ix < 3 || (self.ix < 4 && !self.down) {
            if self.staticcount > 0 {
                // In static phase, no change
            } else if self.rising {
                // Rising (attack or recovery)
                const JUMPTARGET: i32 = 1716;
                if self.level < (JUMPTARGET << 16) {
                    self.level = JUMPTARGET << 16;
                }
                self.level += (((17 << 24) - self.level) >> 24) * self.inc;

                if self.level >= self.targetlevel {
                    self.level = self.targetlevel;
                    self.advance(self.ix + 1);
                }
            } else {
                // Falling (decay, sustain, release)
                self.level -= self.inc;
                if self.level <= self.targetlevel {
                    self.level = self.targetlevel;
                    self.advance(self.ix + 1);
                }
            }
        }

        // Debug: Print envelope values for first few calls (commented out)
        // static mut ENV_DEBUG_COUNT: usize = 0;

        self.level
    }

    /// Handle key down/up events
    pub fn keydown(&mut self, down: bool) {
        if self.down != down {
            self.down = down;
            self.advance(if down { 0 } else { 3 }); // 0=Attack, 3=Release
        }
    }

    /// Scale output level according to DX7 specifications
    pub fn scale_outlevel(outlevel: i32) -> i32 {
        if outlevel >= 20 {
            28 + outlevel
        } else {
            LEVEL_LUT[outlevel as usize]
        }
    }

    /// Get current envelope position
    pub fn get_position(&self) -> i32 {
        self.ix
    }

    /// Transfer state from another envelope (for voice stealing)
    pub fn transfer(&mut self, src: &Env) {
        self.rates = src.rates;
        self.levels = src.levels;
        self.outlevel = src.outlevel;
        self.rate_scaling = src.rate_scaling;
        self.level = src.level;
        self.targetlevel = src.targetlevel;
        self.rising = src.rising;
        self.ix = src.ix;
        self.down = src.down;
        self.staticcount = src.staticcount;
        self.inc = src.inc;
    }

    /// Advance to the next envelope stage
    fn advance(&mut self, newix: i32) {
        self.ix = newix;

        if self.ix < 4 {
            let newlevel = self.levels[self.ix as usize];
            let mut actuallevel = Self::scale_outlevel(newlevel) >> 1;
            actuallevel = (actuallevel << 6) + self.outlevel - 4256;


            actuallevel = if actuallevel < 16 { 16 } else { actuallevel };

            self.targetlevel = actuallevel << 16;
            self.rising = self.targetlevel > self.level;

            // Calculate rate
            let mut qrate = (self.rates[self.ix as usize] * 41) >> 6;
            qrate += self.rate_scaling;
            qrate = min(qrate, 63);

            // Handle static (hold) phases
            if self.targetlevel == self.level || (self.ix == 0 && newlevel == 0) {
                let mut staticrate = self.rates[self.ix as usize];
                staticrate += self.rate_scaling;
                staticrate = min(staticrate, 99);

                self.staticcount = if staticrate < 77 {
                    STATICS[staticrate as usize]
                } else {
                    20 * (99 - staticrate)
                };

                if staticrate < 77 && self.ix == 0 && newlevel == 0 {
                    self.staticcount /= 20; // Attack is scaled faster
                }

                let sr_mult = SR_MULTIPLIER.load(Ordering::Relaxed) as i64;
                self.staticcount = ((self.staticcount as i64 * sr_mult) >> 24) as i32;
            } else {
                self.staticcount = 0;
            }

            // Calculate increment
            self.inc = (4 + (qrate & 3)) << (2 + LG_N + (qrate >> 2) as usize);
            let sr_mult = SR_MULTIPLIER.load(Ordering::Relaxed) as i64;
            self.inc = ((self.inc as i64 * sr_mult) >> 24) as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_creation() {
        let env = Env::new();
        assert_eq!(env.get_position(), 0);
    }

    #[test]
    fn test_env_init() {
        let mut env = Env::new();
        let rates = [50, 50, 50, 50];
        let levels = [99, 75, 50, 0];
        env.init(&rates, &levels, 99, 0);

        // Should start in attack phase
        assert_eq!(env.get_position(), 0);
    }

    #[test]
    fn test_scale_outlevel() {
        assert_eq!(Env::scale_outlevel(0), 0);
        assert_eq!(Env::scale_outlevel(19), 46);
        assert_eq!(Env::scale_outlevel(20), 48); // 28 + 20
        assert_eq!(Env::scale_outlevel(99), 127); // 28 + 99
    }

    #[test]
    fn test_keydown() {
        let mut env = Env::new();
        let rates = [50, 50, 50, 50];
        let levels = [99, 75, 50, 0];
        env.init(&rates, &levels, 99, 0);

        // Start with key down (attack)
        env.keydown(true);
        assert_eq!(env.get_position(), 0);

        // Key up should go to release
        env.keydown(false);
        assert_eq!(env.get_position(), 3);
    }
}