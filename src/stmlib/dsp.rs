// Copyright 2012 Emilie Gillet.
// Rust port by Tyler Neely.
//
// Author: Emilie Gillet (emilie.o.gillet@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// See http://creativecommons.org/licenses/MIT/ for more information.

//! DSP utility functions and unit conversions

use super::sine_lut::LUT_SINE;

/// Linear interpolation in a table
#[inline]
pub fn interpolate(table: &[f32], index: f32, size: f32) -> f32 {
    let index = index * size;
    let index_integral = (index as usize).min(table.len() - 2);
    let index_fractional = index - index_integral as f32;
    let a = table[index_integral];
    let b = table[index_integral + 1];
    a + (b - a) * index_fractional
}

/// Linear interpolation in a table with wrapping
#[inline]
pub fn interpolate_wrap(table: &[f32], mut index: f32, size: f32) -> f32 {
    index -= (index as i32) as f32;
    index *= size;
    let index_integral = index as i32;
    let index_fractional = index - index_integral as f32;
    let a = table[index_integral as usize];
    let b = table[(index_integral + 1) as usize];
    a + (b - a) * index_fractional
}

/// Convert semitones to frequency ratio
#[inline]
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    2.0f32.powf(semitones / 12.0)
}

/// Convert semitones to frequency ratio with safe handling of extreme values
#[inline]
pub fn semitones_to_ratio_safe(mut semitones: f32) -> f32 {
    let mut scale = 1.0f32;
    while semitones > 120.0 {
        semitones -= 120.0;
        scale *= 1024.0;
    }
    while semitones < -120.0 {
        semitones += 120.0;
        scale *= 1.0 / 1024.0;
    }
    scale * semitones_to_ratio(semitones)
}

// Sine oscillator constants and functions
const SINE_LUT_SIZE: f32 = 512.0;
const SINE_LUT_BITS: u32 = 9;

/// Sine lookup with wrapping (safe for phase >= 0.0f)
#[inline]
pub fn sine(phase: f32) -> f32 {
    interpolate_wrap(&LUT_SINE, phase, SINE_LUT_SIZE)
}

/// Phase modulated sine - with positive or negative phase modulation up to an index of 32
#[inline]
pub fn sine_pm(phase: u32, pm: f32) -> f32 {
    const MAX_UINT32: f32 = 4294967296.0;
    const MAX_INDEX: i32 = 32;
    const OFFSET: f32 = MAX_INDEX as f32;
    const SCALE: f32 = MAX_UINT32 / (MAX_INDEX as f32 * 2.0);

    // Use wrapping arithmetic to match C++ unsigned overflow behavior
    let phase_offset = ((pm + OFFSET) * SCALE) as u32;
    let multiplier = (MAX_INDEX as u32).wrapping_mul(2);
    let phase = phase.wrapping_add(phase_offset.wrapping_mul(multiplier));

    let integral = (phase >> (32 - SINE_LUT_BITS)) as usize;
    let fractional = (phase << SINE_LUT_BITS) as f32 / MAX_UINT32;
    let a = LUT_SINE[integral];
    let b = LUT_SINE[integral + 1];
    a + (b - a) * fractional
}
