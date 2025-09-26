
//! Low Frequency Oscillator (LFO) - DX7 compatible
//!
//! The LFO provides modulation for pitch and amplitude with various waveforms
//! and configurable delay. This implementation matches the original DX7 LFO
//! characteristics.

use super::{constants::*, sin::Sin, max};
use std::sync::atomic::{AtomicU32, Ordering};

// LFO frequency lookup table (matches original DX7)
const LFO_SOURCE: [f64; 100] = [
    0.062541, 0.125031, 0.312393, 0.437120, 0.624610,
    0.750694, 0.936330, 1.125302, 1.249609, 1.436782,
    1.560915, 1.752081, 1.875117, 2.062494, 2.247191,
    2.374451, 2.560492, 2.686728, 2.873976, 2.998950,
    3.188013, 3.369840, 3.500175, 3.682224, 3.812065,
    4.000800, 4.186202, 4.310716, 4.501260, 4.623209,
    4.814636, 4.930480, 5.121901, 5.315191, 5.434783,
    5.617346, 5.750431, 5.946717, 6.062811, 6.248438,
    6.431695, 6.564264, 6.749460, 6.868132, 7.052186,
    7.250580, 7.375719, 7.556294, 7.687577, 7.877738,
    7.993605, 8.181967, 8.372405, 8.504848, 8.685079,
    8.810573, 8.986341, 9.122423, 9.300595, 9.500285,
    9.607994, 9.798158, 9.950249, 10.117361, 11.251125,
    11.384335, 12.562814, 13.676149, 13.904338, 15.092062,
    16.366612, 16.638935, 17.869907, 19.193858, 19.425019,
    20.833333, 21.034918, 22.502250, 24.003841, 24.260068,
    25.746653, 27.173913, 27.578599, 29.052876, 30.693677,
    31.191516, 32.658393, 34.317090, 34.674064, 36.416606,
    38.197097, 38.550501, 40.387722, 40.749796, 42.625746,
    44.326241, 44.883303, 46.772685, 48.590865, 49.261084
];

static LFO_UNIT: AtomicU32 = AtomicU32::new(0);
static LFO_RATIO: AtomicU32 = AtomicU32::new(0);

/// LFO waveform types
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum LfoWaveform {
    Triangle = 0,
    SawtoothDown = 1,
    SawtoothUp = 2,
    Square = 3,
    Sine = 4,
    SampleAndHold = 5,
}

impl From<u8> for LfoWaveform {
    fn from(value: u8) -> Self {
        match value {
            0 => LfoWaveform::Triangle,
            1 => LfoWaveform::SawtoothDown,
            2 => LfoWaveform::SawtoothUp,
            3 => LfoWaveform::Square,
            4 => LfoWaveform::Sine,
            5 => LfoWaveform::SampleAndHold,
            _ => LfoWaveform::Triangle, // Default fallback
        }
    }
}

/// DX7-compatible Low Frequency Oscillator
#[derive(Clone, Debug)]
pub struct Lfo {
    phase: u32,          // Q32 phase accumulator
    delta: u32,          // Phase increment per sample
    waveform: LfoWaveform,
    randstate: u8,       // Random state for S&H
    sync: bool,          // Key sync enabled

    // Delay system
    delaystate: u32,
    delayinc: u32,
    delayinc2: u32,
}

impl Default for Lfo {
    fn default() -> Self {
        Self::new()
    }
}

impl Lfo {
    /// Create a new LFO
    pub fn new() -> Self {
        Self {
            phase: 0,
            delta: 0,
            waveform: LfoWaveform::Triangle,
            randstate: 0x80,
            sync: false,
            delaystate: 0,
            delayinc: 0,
            delayinc2: 0,
        }
    }

    /// Initialize LFO timing for the given sample rate
    pub fn init(sample_rate: f64) {
        // constant is 1 << 32 / 15.5s / 11
        let unit = ((N as f64) * 25190424.0 / sample_rate + 0.5) as u32;
        LFO_UNIT.store(unit, Ordering::Relaxed);

        let ratio = 4437500000.0 * (N as f64);
        let lfo_ratio = (ratio / sample_rate) as u32;
        LFO_RATIO.store(lfo_ratio, Ordering::Relaxed);
    }

    /// Reset LFO with DX7 parameters
    ///
    /// # Arguments
    /// * `params` - Array of 6 DX7 LFO parameters:
    ///   - [0]: Rate (0-99)
    ///   - [1]: Delay (0-99)
    ///   - [2]: PMD (Pitch Mod Depth)
    ///   - [3]: AMD (Amplitude Mod Depth)
    ///   - [4]: Sync (0=off, 1=on)
    ///   - [5]: Waveform (0-5)
    pub fn reset(&mut self, params: &[u8; 6]) {
        let rate = params[0] as usize;
        let lfo_ratio = LFO_RATIO.load(Ordering::Relaxed);

        // Clamp rate to valid range
        let rate = rate.min(99);
        self.delta = (LFO_SOURCE[rate] * (lfo_ratio as f64)) as u32;

        // Set up delay
        let delay_param = 99 - params[1]; // LFO delay (inverted)
        let unit = LFO_UNIT.load(Ordering::Relaxed);

        if delay_param == 99 {
            // No delay
            self.delayinc = u32::MAX;
            self.delayinc2 = u32::MAX;
        } else {
            let mut a = (16 + (delay_param & 15)) << (1 + (delay_param >> 4));
            self.delayinc = unit.wrapping_mul(a as u32);
            a = (a as u32 & 0xff80) as u8;
            a = max(0x80, a);
            self.delayinc2 = unit.wrapping_mul(a as u32);
        }

        self.waveform = LfoWaveform::from(params[5]);
        self.sync = params[4] != 0;
    }

    /// Get the next LFO sample
    ///
    /// Returns a value in Q24 format (0..1 scaled to 0..16777216)
    pub fn get_sample(&mut self) -> i32 {
        self.phase = self.phase.wrapping_add(self.delta);

        match self.waveform {
            LfoWaveform::Triangle => {
                let mut x = self.phase >> 7;
                x ^= ((self.phase as i32) >> 31) as u32;
                x &= (1 << 24) - 1;
                x as i32
            }
            LfoWaveform::SawtoothDown => {
                (((!self.phase) ^ (1u32 << 31)) >> 8) as i32
            }
            LfoWaveform::SawtoothUp => {
                ((self.phase ^ (1u32 << 31)) >> 8) as i32
            }
            LfoWaveform::Square => {
                (((!self.phase) >> 7) & (1u32 << 24)) as i32
            }
            LfoWaveform::Sine => {
                (1 << 23) + (Sin::lookup((self.phase >> 8) as i32) >> 1)
            }
            LfoWaveform::SampleAndHold => {
                if self.phase < self.delta {
                    self.randstate = (self.randstate.wrapping_mul(179).wrapping_add(17)) & 0xff;
                }
                let x = self.randstate ^ 0x80;
                ((x as i32) + 1) << 16
            }
        }
    }

    /// Get the current delay amount
    ///
    /// Returns a value in Q24 format representing the delay envelope
    pub fn get_delay(&mut self) -> i32 {
        let delta = if self.delaystate < (1u32 << 31) {
            self.delayinc
        } else {
            self.delayinc2
        };

        let d = (self.delaystate as u64).wrapping_add(delta as u64);
        if d > u32::MAX as u64 {
            return 1 << 24;
        }

        self.delaystate = d as u32;
        if d < (1u64 << 31) {
            0
        } else {
            ((d >> 7) & ((1u64 << 24) - 1)) as i32
        }
    }

    /// Handle key down event
    pub fn keydown(&mut self) {
        if self.sync {
            self.phase = (1u32 << 31).wrapping_sub(1);
        }
        self.delaystate = 0;
    }

    /// Get current waveform
    pub fn waveform(&self) -> LfoWaveform {
        self.waveform
    }

    /// Set waveform directly
    pub fn set_waveform(&mut self, waveform: LfoWaveform) {
        self.waveform = waveform;
    }

    /// Get sync setting
    pub fn sync(&self) -> bool {
        self.sync
    }

    /// Set sync setting
    pub fn set_sync(&mut self, sync: bool) {
        self.sync = sync;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfo_creation() {
        let lfo = Lfo::new();
        assert_eq!(lfo.waveform(), LfoWaveform::Triangle);
        assert!(!lfo.sync());
    }

    #[test]
    fn test_lfo_init() {
        Lfo::init(44100.0);
        // Should not panic, values should be set
        let unit = LFO_UNIT.load(Ordering::Relaxed);
        let ratio = LFO_RATIO.load(Ordering::Relaxed);
        assert_ne!(unit, 0);
        assert_ne!(ratio, 0);
    }

    #[test]
    fn test_lfo_reset() {
        Lfo::init(44100.0);
        let mut lfo = Lfo::new();

        // Test parameters: rate=50, delay=0, pmd=0, amd=0, sync=1, wave=4(sine)
        let params = [50, 0, 0, 0, 1, 4];
        lfo.reset(&params);

        assert_eq!(lfo.waveform(), LfoWaveform::Sine);
        assert!(lfo.sync());
    }

    #[test]
    fn test_lfo_waveforms() {
        Lfo::init(44100.0);
        let mut lfo = Lfo::new();

        // Test each waveform
        for waveform in 0..6 {
            let params = [50, 0, 0, 0, 0, waveform];
            lfo.reset(&params);

            // Get a few samples to ensure no panics
            for _ in 0..10 {
                let _sample = lfo.get_sample();
            }
        }
    }

    #[test]
    fn test_waveform_from() {
        assert_eq!(LfoWaveform::from(0), LfoWaveform::Triangle);
        assert_eq!(LfoWaveform::from(1), LfoWaveform::SawtoothDown);
        assert_eq!(LfoWaveform::from(2), LfoWaveform::SawtoothUp);
        assert_eq!(LfoWaveform::from(3), LfoWaveform::Square);
        assert_eq!(LfoWaveform::from(4), LfoWaveform::Sine);
        assert_eq!(LfoWaveform::from(5), LfoWaveform::SampleAndHold);
        assert_eq!(LfoWaveform::from(99), LfoWaveform::Triangle); // Fallback
    }

    #[test]
    fn test_keydown() {
        let mut lfo = Lfo::new();
        lfo.set_sync(true);
        lfo.keydown();
        // Should reset phase when sync is enabled
        assert_eq!(lfo.delaystate, 0);
    }

    #[test]
    fn test_delay_envelope() {
        Lfo::init(44100.0);
        let mut lfo = Lfo::new();

        // Set up with some delay - params [rate, pitch_mod_depth, amp_mod_depth, delay, pmd_pms, ams, sync]
        let params = [50, 50, 0, 80, 0, 0]; // High delay value
        lfo.reset(&params);

        // Initially should return 0 (full delay)
        let initial_delay = lfo.get_delay();
        assert_eq!(initial_delay, 0);

        // After many samples, delay should increase
        for _ in 0..100000 {  // Need more samples for delay envelope to progress
            lfo.get_delay();
        }
        let later_delay = lfo.get_delay();
        assert!(later_delay > initial_delay, "Expected later_delay {} > initial_delay {}", later_delay, initial_delay);
    }
}