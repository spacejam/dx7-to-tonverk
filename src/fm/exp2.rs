
//! Exponential (2^x) lookup table and computation
//!
//! Used for converting logarithmic envelope values to linear gain values
//! in the FM synthesis engine.

/// Exponential lookup and computation
pub struct Exp2;

impl Exp2 {
    /// Compute 2^x using approximation
    ///
    /// # Arguments
    /// * `x` - Input value in Q24 format (24-bit fractional part)
    ///
    /// # Returns
    /// 2^x in Q24 fixed-point format
    pub fn lookup(x: i32) -> i32 {
        // Handle the common case where x is very negative (envelope at low level)
        if x < -20 * (1 << 24) {
            return 0; // Effectively silent
        }

        // Convert from Q24 to floating point
        let x_float = (x as f64) / ((1 << 24) as f64);

        // Compute 2^x
        let result_float = 2.0_f64.powf(x_float);

        // Convert back to Q24 format with saturation
        let result_q24 = (result_float * ((1 << 24) as f64)) as i64;

        // Clamp to valid range for i32
        result_q24.clamp(0, i32::MAX as i64) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exp2_lookup() {
        // Basic functionality test
        let result = Exp2::lookup(0);
        assert_eq!(result, 1 << 24); // 2^0 = 1

        // Test negative input (should be 0.5)
        let result = Exp2::lookup(-1 << 24);
        assert_eq!(result, 1 << 23); // 2^(-1) = 0.5
    }
}