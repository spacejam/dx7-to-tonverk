
//! FM operator kernel - the core of FM synthesis
//!
//! This module implements the basic building blocks of FM synthesis:
//! - Basic FM operators (with modulation input)
//! - Pure sine wave generators (no modulation)
//! - Feedback operators (self-modulation)

use super::{constants::*, sin::Sin};
use log::trace;

/// Parameters for FM operator computation
#[derive(Clone, Debug)]
pub struct FmOpParams {
    pub level_in: i32,  // Input level (from envelope)
    pub gain_out: i32,  // Output gain (computed)
    pub freq: i32,      // Frequency (phase increment)
    pub phase: i32,     // Current phase
}

/// FM operator kernel - provides the core FM synthesis algorithms
pub struct FmOpKernel;

impl FmOpKernel {
    /// Compute FM operator with modulation input
    ///
    /// This is the basic FM operator that takes a modulation input and produces
    /// a modulated sine wave output. The gain linearly interpolates from gain1 to gain2
    /// over the N samples.
    ///
    /// # Arguments
    /// * `output` - Output buffer (N samples)
    /// * `input` - Modulation input buffer (N samples)
    /// * `phase0` - Starting phase
    /// * `freq` - Frequency (phase increment per sample)
    /// * `gain1` - Starting gain
    /// * `gain2` - Ending gain
    /// * `add` - Whether to add to existing output or replace
    pub fn compute(
        output: &mut [i32],
        input: &[i32],
        phase0: i32,
        freq: i32,
        gain1: i32,
        gain2: i32,
        add: bool,
    ) {
        assert_eq!(output.len(), N);
        assert_eq!(input.len(), N);

        let dgain = (gain2 - gain1 + ((N >> 1) as i32)) >> LG_N;
        let mut gain = gain1;
        let mut phase = phase0;

        if add {
            for i in 0..N {
                gain += dgain;
                let y = Sin::lookup(phase + input[i]);
                let y1 = ((y as i64) * (gain as i64)) >> 24;
                output[i] += y1 as i32;
                phase += freq;
            }
        } else {
            for i in 0..N {
                gain += dgain;
                let y = Sin::lookup(phase + input[i]);
                let y1 = ((y as i64) * (gain as i64)) >> 24;
                output[i] = y1 as i32;
                phase += freq;
            }
        }
    }

    /// Compute pure sine wave (no modulation input)
    ///
    /// This generates a pure sine wave with no frequency modulation input.
    /// Used for carriers and oscillators that don't need modulation.
    ///
    /// # Arguments
    /// * `output` - Output buffer (N samples)
    /// * `phase0` - Starting phase
    /// * `freq` - Frequency (phase increment per sample)
    /// * `gain1` - Starting gain
    /// * `gain2` - Ending gain
    /// * `add` - Whether to add to existing output or replace
    pub fn compute_pure(
        output: &mut [i32],
        phase0: i32,
        freq: i32,
        gain1: i32,
        gain2: i32,
        add: bool,
    ) {
        assert_eq!(output.len(), N);


        let dgain = (gain2 - gain1 + ((N >> 1) as i32)) >> LG_N;
        let mut gain = gain1;
        let mut phase = phase0;

        if add {
            for i in 0..N {
                gain += dgain;
                let y = Sin::lookup(phase);
                let y1 = ((y as i64) * (gain as i64)) >> 24;
                output[i] += y1 as i32;
                phase += freq;
            }
        } else {
            for i in 0..N {
                gain += dgain;
                let y = Sin::lookup(phase);
                let y1 = ((y as i64) * (gain as i64)) >> 24;
                output[i] = y1 as i32;

                // Debug first few samples
                static mut DEBUG_COUNT: usize = 0;
                unsafe {
                    DEBUG_COUNT += 1;
                    if DEBUG_COUNT <= 5 {
                        trace!("SINE DEBUG {}: phase={}, freq={}, y={}, gain={}, y1={}, output={}",
                            DEBUG_COUNT, phase, freq, y, gain, y1, output[i]);
                    }
                }


                phase += freq;
            }
        }
    }

    /// Compute FM operator with feedback
    ///
    /// This implements self-modulation (feedback) where the operator's output
    /// modulates its own frequency. This creates rich harmonic content and is
    /// essential for many classic FM sounds.
    ///
    /// # Arguments
    /// * `output` - Output buffer (N samples)
    /// * `phase0` - Starting phase
    /// * `freq` - Frequency (phase increment per sample)
    /// * `gain1` - Starting gain
    /// * `gain2` - Ending gain
    /// * `fb_buf` - Feedback buffer [y0, y1] (modified in-place)
    /// * `fb_shift` - Feedback amount (right shift amount)
    /// * `add` - Whether to add to existing output or replace
    pub fn compute_fb(
        output: &mut [i32],
        phase0: i32,
        freq: i32,
        gain1: i32,
        gain2: i32,
        fb_buf: &mut [i32; 2],
        fb_shift: i32,
        add: bool,
    ) {
        assert_eq!(output.len(), N);

        let dgain = (gain2 - gain1 + ((N >> 1) as i32)) >> LG_N;
        let mut gain = gain1;
        let mut phase = phase0;
        let mut y0 = fb_buf[0];
        let mut y = fb_buf[1];

        if add {
            for i in 0..N {
                gain += dgain;
                let shift_amount = (fb_shift + 1).min(31); // Clamp to prevent overflow
                let scaled_fb = (y0 + y) >> shift_amount;
                y0 = y;
                y = Sin::lookup(phase + scaled_fb);
                y = (((y as i64) * (gain as i64)) >> 24) as i32;
                output[i] += y as i32;
                phase += freq;
            }
        } else {
            for i in 0..N {
                gain += dgain;
                let shift_amount = (fb_shift + 1).min(31); // Clamp to prevent overflow
                let scaled_fb = (y0 + y) >> shift_amount;
                y0 = y;
                y = Sin::lookup(phase + scaled_fb);
                y = (((y as i64) * (gain as i64)) >> 24) as i32;
                output[i] = y as i32;
                phase += freq;
            }
        }

        fb_buf[0] = y0;
        fb_buf[1] = y;
    }

    /// Convenience method to zero a buffer
    pub fn zero_buffer(buffer: &mut [i32]) {
        buffer.fill(0);
    }

    /// Convenience method to scale a buffer by a gain value
    pub fn scale_buffer(buffer: &mut [i32], gain: i32) {
        for sample in buffer.iter_mut() {
            *sample = (((*sample as i64) * (gain as i64)) >> 24) as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_pure() {
        let mut output = [0i32; N];
        let phase0 = 0;
        let freq = 1 << 20; // Some reasonable frequency
        let gain1 = 1 << 24; // Full gain
        let gain2 = 1 << 24;

        FmOpKernel::compute_pure(&mut output, phase0, freq, gain1, gain2, false);

        // Output should not be all zeros - check multiple samples
        let has_nonzero = output.iter().any(|&x| x != 0);
        assert!(has_nonzero, "Expected at least one non-zero output sample");
    }

    #[test]
    fn test_compute_with_modulation() {
        let mut output = [0i32; N];
        let input = [1 << 20; N]; // Constant modulation
        let phase0 = 0;
        let freq = 1 << 20;
        let gain1 = 1 << 24;
        let gain2 = 1 << 24;

        FmOpKernel::compute(&mut output, &input, phase0, freq, gain1, gain2, false);

        // Output should not be all zeros - check multiple samples
        let has_nonzero = output.iter().any(|&x| x != 0);
        assert!(has_nonzero, "Expected at least one non-zero output sample");
    }

    #[test]
    fn test_compute_fb() {
        let mut output = [0i32; N];
        let mut fb_buf = [0i32; 2];
        let phase0 = 0;
        let freq = 1 << 20;
        let gain1 = 1 << 24;
        let gain2 = 1 << 24;
        let fb_shift = 4; // Moderate feedback

        FmOpKernel::compute_fb(
            &mut output, phase0, freq, gain1, gain2, &mut fb_buf, fb_shift, false
        );

        // Output should not be all zeros - check multiple samples
        let has_nonzero = output.iter().any(|&x| x != 0);
        assert!(has_nonzero, "Expected at least one non-zero output sample");

        // Feedback buffer should be updated (they now contain the last two output values)
        assert_ne!(fb_buf[0], 0);
        assert_ne!(fb_buf[1], 0);
    }

    #[test]
    fn test_add_mode() {
        let mut output = [100i32; N]; // Pre-filled buffer
        let phase0 = 0;
        let freq = 1 << 20;
        let gain1 = 1 << 24;
        let gain2 = 1 << 24;

        let original_values: Vec<i32> = output.to_vec();
        FmOpKernel::compute_pure(&mut output, phase0, freq, gain1, gain2, true);

        // Check if any values were modified (added to)
        let values_changed = output.iter().zip(original_values.iter())
            .any(|(&new, &old)| new != old);
        assert!(values_changed, "Expected some output values to be modified in add mode");
    }

    #[test]
    fn test_zero_buffer() {
        let mut buffer = [42i32; N];
        FmOpKernel::zero_buffer(&mut buffer);

        for &sample in &buffer {
            assert_eq!(sample, 0);
        }
    }

    #[test]
    fn test_scale_buffer() {
        let mut buffer = [1 << 24; N]; // Fill with full-scale values
        let gain = 1 << 23; // Half gain

        FmOpKernel::scale_buffer(&mut buffer, gain);

        // Values should be approximately halved
        for &sample in &buffer {
            assert_eq!(sample, 1 << 23);
        }
    }
}