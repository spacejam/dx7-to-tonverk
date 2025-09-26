
//! High-performance sine wave computation using lookup tables and polynomial approximation

use std::sync::Once;

const SIN_LG_N_SAMPLES: usize = 10;
const SIN_N_SAMPLES: usize = 1 << SIN_LG_N_SAMPLES;
const R: i32 = 1 << 29;

// Use SIN_DELTA optimization for better performance
static mut SINTAB: [i32; SIN_N_SAMPLES << 1] = [0; SIN_N_SAMPLES << 1];
static INIT_ONCE: Once = Once::new();

pub struct Sin;

impl Sin {
    /// Initialize the sine lookup table
    pub fn init() {
        INIT_ONCE.call_once(|| unsafe {
            Self::build_table();
        });
    }

    unsafe fn build_table() {
        let dphase = 2.0 * std::f64::consts::PI / (SIN_N_SAMPLES as f64);
        let c = (dphase.cos() * (1i64 << 30) as f64 + 0.5).floor() as i32;
        let s = (dphase.sin() * (1i64 << 30) as f64 + 0.5).floor() as i32;

        let mut u = 1i32 << 30;
        let mut v = 0i32;

        for i in 0..(SIN_N_SAMPLES / 2) {
            // SIN_DELTA version
            SINTAB[(i << 1) + 1] = (v + 32) >> 6;
            SINTAB[((i + SIN_N_SAMPLES / 2) << 1) + 1] = -((v + 32) >> 6);

            let t = ((u as i64) * (s as i64) + (v as i64) * (c as i64) + R as i64) >> 30;
            u = ((u as i64) * (c as i64) - (v as i64) * (s as i64) + R as i64) as i32;
            v = t as i32;
        }

        // Build delta table
        for i in 0..(SIN_N_SAMPLES - 1) {
            SINTAB[i << 1] = SINTAB[(i << 1) + 3] - SINTAB[(i << 1) + 1];
        }
        SINTAB[(SIN_N_SAMPLES << 1) - 2] = -SINTAB[(SIN_N_SAMPLES << 1) - 1];
    }

    /// Fast sine lookup with linear interpolation
    #[inline]
    pub fn lookup(phase: i32) -> i32 {
        Self::init(); // Ensure table is initialized

        const SHIFT: i32 = 24 - SIN_LG_N_SAMPLES as i32;
        let lowbits = phase & ((1 << SHIFT) - 1);

        unsafe {
            // SIN_DELTA version
            let phase_int = ((phase >> (SHIFT - 1)) & ((SIN_N_SAMPLES - 1) << 1) as i32) as usize;
            let dy = SINTAB[phase_int];
            let y0 = SINTAB[phase_int + 1];

            y0 + (((dy as i64) * (lowbits as i64)) >> SHIFT) as i32
        }
    }

    /// Compute sine using Chebyshev polynomial approximation
    pub fn compute(phase: i32) -> i32 {
        // Chebyshev polynomial coefficients
        const C8_0: i32 = 16777216;
        const C8_2: i32 = -331168742;
        const C8_4: i32 = 1089453524;
        const C8_6: i32 = -1430910663;
        const C8_8: i32 = 950108533;

        let x = (phase & ((1 << 23) - 1)) - (1 << 22);
        let x2 = ((x as i64) * (x as i64)) >> 16;

        let mut y = (((((((((((((C8_8 as i64)
            * x2) >> 32) + C8_6 as i64)
            * x2) >> 32) + C8_4 as i64)
            * x2) >> 32) + C8_2 as i64)
            * x2) >> 32) + C8_0 as i64) as i32;

        y ^= -((phase >> 23) & 1);
        y
    }

    /// More accurate sine computation (Q30 input and output)
    pub fn compute10(phase: i32) -> i32 {
        const C10_0: i32 = 1 << 30;
        const C10_2: i32 = -1324675874;
        const C10_4: i32 = 1089501821;
        const C10_6: i32 = -1433689867;
        const C10_8: i32 = 1009356886;
        const C10_10: i32 = -421101352;

        let x = (phase & ((1 << 29) - 1)) - (1 << 28);
        let x2 = ((x as i64) * (x as i64)) >> 26;

        let mut y = ((((((((((((((((C10_10 as i64)
            * x2) >> 34) + C10_8 as i64)
            * x2) >> 34) + C10_6 as i64)
            * x2) >> 34) + C10_4 as i64)
            * x2) >> 32) + C10_2 as i64)
            * x2) >> 30) + C10_0 as i64) as i32;

        y ^= -((phase >> 29) & 1);
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sin_init() {
        Sin::init();
        // Table should be initialized without panicking
    }

    #[test]
    fn test_sin_lookup() {
        let phase = 1 << 22; // Quarter phase
        let result = Sin::lookup(phase);
        // Should be approximately maximum positive value
        assert!(result > 0);
    }

    #[test]
    fn test_sin_compute() {
        let phase = 1 << 22;
        let result = Sin::compute(phase);
        assert!(result > 0);
    }
}