
//! Frequency lookup table for converting logarithmic frequency to phase increment
//!
//! This is a direct port of the Dexed/MSFA Freqlut functionality that converts
//! logarithmic frequency (Q24 format) to phase increment values for oscillators.

use std::sync::Once;

const LG_N_SAMPLES: i32 = 10;
const N_SAMPLES: usize = 1 << LG_N_SAMPLES; // 1024
const SAMPLE_SHIFT: i32 = 24 - LG_N_SAMPLES; // 14
const MAX_LOGFREQ_INT: i32 = 20;

static mut LUT: [i32; N_SAMPLES + 1] = [0; N_SAMPLES + 1];
static INIT: Once = Once::new();

/// Frequency lookup table (exact Dexed/MSFA port)
pub struct Freqlut;

impl Freqlut {
    /// Initialize the frequency lookup table for given sample rate
    /// This must be called once before using lookup()
    pub fn init(sample_rate: f64) {
        INIT.call_once(|| {
            unsafe {
                let mut y = ((1i64 << (24 + MAX_LOGFREQ_INT)) as f64) / sample_rate;
                let inc = 2.0f64.powf(1.0 / N_SAMPLES as f64);

                for i in 0..=N_SAMPLES {
                    LUT[i] = (y + 0.5).floor() as i32;
                    y *= inc;
                }
            }
        });
    }

    /// Convert logarithmic frequency (Q24 format) to phase increment
    ///
    /// This is an exact port of the Dexed Freqlut::lookup() function.
    ///
    /// # Arguments
    /// * `logfreq` - Logarithmic frequency in Q24 format where 1.0 = 1 octave
    ///
    /// # Returns
    /// Phase increment value suitable for FM synthesis
    ///
    /// # Note
    /// If logfreq is more than 20.0, the results will be inaccurate. However,
    /// that will be many times the Nyquist rate.
    pub fn lookup(logfreq: i32) -> i32 {
        unsafe {
            let ix = ((logfreq & 0xffffff) >> SAMPLE_SHIFT) as usize;
            if ix >= N_SAMPLES {
                return 0; // Prevent out of bounds access
            }

            let y0 = LUT[ix];
            let y1 = LUT[ix + 1];
            let lowbits = logfreq & ((1 << SAMPLE_SHIFT) - 1);
            let y = y0 + (((y1 as i64 - y0 as i64) * lowbits as i64) >> SAMPLE_SHIFT) as i32;
            let hibits = logfreq >> 24;

            let shift = MAX_LOGFREQ_INT - hibits;
            if shift < 0 {
                // If hibits > MAX_LOGFREQ_INT, clamp to a high frequency
                y << (-shift).min(31) // Limit shift to prevent overflow
            } else {
                y >> shift.min(31) // Limit shift to prevent overflow
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freqlut_init() {
        Freqlut::init(44100.0);
        // After initialization, the lookup table should be populated
        unsafe {
            assert_ne!(LUT[0], 0);
            assert_ne!(LUT[N_SAMPLES], 0);
        }
    }

    #[test]
    fn test_freqlut_lookup() {
        Freqlut::init(44100.0);

        // Test with a reasonable logarithmic frequency value
        let logfreq = 1 << 24; // 1.0 in Q24 format (1 octave)
        let phase_inc = Freqlut::lookup(logfreq);

        // Phase increment should be non-zero for valid input
        assert_ne!(phase_inc, 0);
    }

    #[test]
    fn test_freqlut_boundary() {
        Freqlut::init(44100.0);

        // Test boundary conditions
        let zero_result = Freqlut::lookup(0);
        assert_ne!(zero_result, 0);

        // Test very large value (should be clamped)
        let large_result = Freqlut::lookup(i32::MAX);
        // Should not crash and return some reasonable value
        let _ = large_result;
    }
}