// Copyright 2025 Tyler Neely (tylerneely@gmail.com).
// Copyright 2021 Emilie Gillet (emilie.o.gillet@gmail.com)
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

//! FM operator with phase accumulation and sine generation

use crate::stmlib::dsp::sine_pm;

/// Modulation source identifiers for operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModulationSource {
    /// External modulation source (-2)
    External = -2,
    /// No modulation (-1)
    None = -1,
    /// Feedback modulation (0)
    Feedback = 0,
}

/// FM operator state
#[derive(Debug, Clone, Copy)]
pub struct Operator {
    /// Phase accumulator (32-bit unsigned for wraparound)
    pub phase: u32,
    /// Current amplitude
    pub amplitude: f32,
}

impl Operator {
    /// Resets the operator state
    #[inline]
    pub fn reset(&mut self) {
        self.phase = 0;
        self.amplitude = 0.0;
    }
}

impl Default for Operator {
    fn default() -> Self {
        Self {
            phase: 0,
            amplitude: 0.0,
        }
    }
}

/// Function pointer type for operator rendering
pub type RenderFn = fn(
    ops: &mut [Operator],
    f: &[f32],
    a: &[f32],
    fb_state: &mut [f32; 2],
    fb_amount: i32,
    modulation: &[f32],
    out: &mut [f32],
);

/// Renders a chain of operators with specified modulation source
pub fn render_operators<const N: usize, const MODULATION_SOURCE: i32, const ADDITIVE: bool>(
    ops: &mut [Operator],
    f: &[f32],
    a: &[f32],
    fb_state: &mut [f32; 2],
    fb_amount: i32,
    modulation: &[f32],
    out: &mut [f32],
) {
    let size = out.len();
    let mut previous_0 = 0.0f32;
    let mut previous_1 = 0.0f32;

    if MODULATION_SOURCE >= 0 {
        previous_0 = fb_state[0];
        previous_1 = fb_state[1];
    }

    let mut frequency = [0u32; N];
    let mut phase = [0u32; N];
    let mut amplitude = [0.0f32; N];
    let mut amplitude_increment = [0.0f32; N];

    let scale = 1.0 / size as f32;
    for i in 0..N {
        frequency[i] = (f[i].min(0.5) * 4294967296.0) as u32;
        phase[i] = ops[i].phase;
        amplitude[i] = ops[i].amplitude;
        amplitude_increment[i] = (a[i].min(4.0) - amplitude[i]) * scale;
    }

    let fb_scale = if fb_amount != 0 {
        (1 << fb_amount) as f32 / 512.0
    } else {
        0.0
    };

    let mut mod_idx = 0;
    for sample_idx in 0..size {
        let mut pm = if MODULATION_SOURCE >= 0 {
            (previous_0 + previous_1) * fb_scale
        } else if MODULATION_SOURCE == -2 {
            modulation[mod_idx]
        } else {
            0.0
        };

        if MODULATION_SOURCE == -2 {
            mod_idx += 1;
        }

        for i in 0..N {
            phase[i] = phase[i].wrapping_add(frequency[i]);
            pm = sine_pm(phase[i], pm) * amplitude[i];
            amplitude[i] += amplitude_increment[i];
            if i as i32 == MODULATION_SOURCE {
                previous_1 = previous_0;
                previous_0 = pm;
            }
        }

        if ADDITIVE {
            out[sample_idx] += pm;
        } else {
            out[sample_idx] = pm;
        }
    }

    for i in 0..N {
        ops[i].phase = phase[i];
        ops[i].amplitude = amplitude[i];
    }

    if MODULATION_SOURCE >= 0 {
        fb_state[0] = previous_0;
        fb_state[1] = previous_1;
    }
}