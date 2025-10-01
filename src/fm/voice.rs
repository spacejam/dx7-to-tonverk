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

//! DX7 voice - main synthesis entry point

use super::algorithms::Algorithms;
use super::dx_units::{
    amp_mod_sensitivity, frequency_ratio, keyboard_scaling, normalize_velocity, operator_level,
    pow2_fast, rate_scaling,
};
use super::envelope::{OperatorEnvelope, PitchEnvelope};
use super::operator::Operator;
use super::patch::Patch;

use crate::stmlib::dsp::semitones_to_ratio_safe;
use crate::NUM_OPERATORS;

/// Voice parameters for rendering
pub struct Parameters {
    /// Sustain mode (envelope scrubbing)
    pub sustain: bool,
    /// Gate signal (note on/off)
    pub gate: bool,
    /// MIDI note number
    pub note: f32,
    /// Velocity (0.0-1.0)
    pub velocity: f32,
    /// Brightness control (affects modulator levels)
    pub brightness: f32,
    /// Envelope time control (0.0-1.0, 0.5 = normal)
    pub envelope_control: f32,
    /// Pitch modulation amount
    pub pitch_mod: f32,
    /// Amplitude modulation amount
    pub amp_mod: f32,
}

impl Default for Parameters {
    fn default() -> Parameters {
        Parameters {
            sustain: false,
            gate: false,
            note: 48.0,
            velocity: 0.5,
            brightness: 0.5,
            envelope_control: 0.5,
            pitch_mod: 0.0,
            amp_mod: 0.0,
        }
    }
}

/// DX7 FM voice
pub struct Voice {
    algorithms: Algorithms,
    sample_rate: f32,
    one_hz: f32,
    a0: f32,
    gate: bool,
    operator: [Operator; NUM_OPERATORS],
    operator_envelope: [OperatorEnvelope; NUM_OPERATORS],
    pitch_envelope: PitchEnvelope,
    normalized_velocity: f32,
    note: f32,
    ratios: [f32; NUM_OPERATORS],
    level_headroom: [f32; NUM_OPERATORS],
    level: [f32; NUM_OPERATORS],
    feedback_state: [f32; 2],
    patch: Patch,
    dirty: bool,
}

impl Voice {
    /// Creates a new voice
    pub fn new(patch: Patch, sample_rate: f32) -> Self {
        let mut ret = Self {
            algorithms: Algorithms::new(),
            sample_rate,
            one_hz: 1.0 / sample_rate,
            a0: 55.0 / sample_rate,
            gate: false,
            operator: [Operator::default(); NUM_OPERATORS],
            operator_envelope: [OperatorEnvelope::new(); NUM_OPERATORS],
            pitch_envelope: PitchEnvelope::new(),
            normalized_velocity: 10.0,
            note: 48.0,
            ratios: [0.0; NUM_OPERATORS],
            level_headroom: [0.0; NUM_OPERATORS],
            level: [0.0; NUM_OPERATORS],
            feedback_state: [0.0, 0.0],
            patch,
            dirty: true,
        };

        let native_sr = 44100.0;
        let envelope_scale = native_sr * ret.one_hz;

        for i in 0..NUM_OPERATORS {
            ret.operator[i].reset();
            ret.operator_envelope[i].init(envelope_scale);
        }
        ret.pitch_envelope.init(envelope_scale);
        ret.setup();

        ret
    }

    /// Pre-computes patch-dependent data
    fn setup(&mut self) -> bool {
        if !self.dirty {
            return false;
        }

        self.pitch_envelope.set(
            &self.patch.pitch_envelope.rate,
            &self.patch.pitch_envelope.level,
        );

        for i in 0..NUM_OPERATORS {
            let op = &self.patch.op[i];
            let level = operator_level(op.level as i32);
            self.operator_envelope[i].set(&op.envelope.rate, &op.envelope.level, level as u8);
            self.level_headroom[i] = (127 - level) as f32;
            let sign = if op.mode == 0 { 1.0 } else { -1.0 };
            self.ratios[i] = sign * frequency_ratio(op);
        }

        self.dirty = false;
        true
    }

    /// Returns the level of an operator
    #[inline]
    pub fn op_level(&self, i: usize) -> f32 {
        self.level[i]
    }

    /// Renders audio with 2 output buffers (out and aux)
    pub fn render_stereo(
        &mut self,
        parameters: &Parameters,
        temp: &mut [f32],
        out: &mut [f32],
        aux: &mut [f32],
    ) {
        let size = out.len();
        let mut buffers = [
            out.as_mut_ptr(),
            aux.as_mut_ptr(),
            temp.as_mut_ptr(),
            temp[size..].as_mut_ptr(),
        ];
        self.render_internal(parameters, &mut buffers, size);
    }

    /// Renders audio with single temp buffer
    pub fn render_temp(&mut self, parameters: &Parameters, temp: &mut [f32]) {
        let size = temp.len() / 3;
        let mut buffers = [
            temp.as_mut_ptr(),
            unsafe { temp.as_mut_ptr().add(size) },
            unsafe { temp.as_mut_ptr().add(2 * size) },
            unsafe { temp.as_mut_ptr().add(2 * size) },
        ];
        self.render_internal(parameters, &mut buffers, size);
    }

    fn render_internal(
        &mut self,
        parameters: &Parameters,
        buffers: &mut [*mut f32; 4],
        size: usize,
    ) {
        if self.setup() {
            return;
        }

        let envelope_rate = size as f32;
        let ad_scale = pow2_fast::<1>((0.5 - parameters.envelope_control) * 8.0);
        let r_scale = pow2_fast::<1>(-(parameters.envelope_control - 0.3).abs() * 8.0);
        let gate_duration = 1.5 * self.sample_rate;
        let envelope_sample = gate_duration * parameters.envelope_control;

        let input_note = parameters.note - 24.0 + self.patch.transpose as f32;

        let pitch_envelope = if parameters.sustain {
            self.pitch_envelope
                .render_at_sample(envelope_sample, gate_duration)
        } else {
            self.pitch_envelope
                .render_scaled(parameters.gate, envelope_rate, ad_scale, r_scale)
        };

        let pitch_mod = pitch_envelope + parameters.pitch_mod;
        let f0 = self.a0 * 0.25 * semitones_to_ratio_safe(input_note - 9.0 + pitch_mod * 12.0);

        let note_on = parameters.gate && !self.gate;
        self.gate = parameters.gate;
        if note_on || parameters.sustain {
            self.normalized_velocity = normalize_velocity(parameters.velocity);
            self.note = input_note;
        }

        if note_on && self.patch.reset_phase != 0 {
            for i in 0..NUM_OPERATORS {
                self.operator[i].phase = 0;
            }
        }

        let mut f = [0.0f32; NUM_OPERATORS];
        let mut a = [0.0f32; NUM_OPERATORS];

        for i in 0..NUM_OPERATORS {
            let op = &self.patch.op[i];
            f[i] = self.ratios[i]
                * if self.ratios[i] < 0.0 {
                    -self.one_hz
                } else {
                    f0
                };

            let rate_scaling_val = rate_scaling(self.note, op.rate_scaling as i32);
            let level = if parameters.sustain {
                self.operator_envelope[i].render_at_sample(envelope_sample, gate_duration)
            } else {
                self.operator_envelope[i].render_scaled(
                    parameters.gate,
                    envelope_rate * rate_scaling_val,
                    ad_scale,
                    r_scale,
                )
            };

            let kb_scaling = keyboard_scaling(self.note, &op.keyboard_scaling);
            let velocity_scaling = self.normalized_velocity * op.velocity_sensitivity as f32;
            let brightness = if self
                .algorithms
                .is_modulator(self.patch.algorithm as usize, i)
            {
                (parameters.brightness - 0.5) * 32.0
            } else {
                0.0
            };

            let level = level
                + 0.125 * (kb_scaling + velocity_scaling + brightness).min(self.level_headroom[i]);
            self.level[i] = level;

            let sensitivity = amp_mod_sensitivity(op.amp_mod_sensitivity as i32);
            #[cfg(feature = "fast_op_level_modulation")]
            {
                let level_mod = 1.0 - sensitivity * parameters.amp_mod;
                a[i] = pow2_fast::<2>(-14.0 + level) * level_mod;
            }
            #[cfg(not(feature = "fast_op_level_modulation"))]
            {
                let log_level_mod = sensitivity * parameters.amp_mod - 1.0;
                let level_mod = 1.0 - pow2_fast::<2>(6.4 * log_level_mod);
                a[i] = pow2_fast::<2>(-14.0 + level * level_mod);
            }
        }

        let mut i = 0;
        while i < NUM_OPERATORS {
            let call = self
                .algorithms
                .render_call(self.patch.algorithm as usize, i);
            let ops_slice = &mut self.operator[i..i + call.n];
            let f_slice = &f[i..i + call.n];
            let a_slice = &a[i..i + call.n];

            let input_buffer =
                unsafe { std::slice::from_raw_parts(buffers[call.input_index], size) };
            let output_buffer =
                unsafe { std::slice::from_raw_parts_mut(buffers[call.output_index], size) };

            (call.render_fn)(
                ops_slice,
                f_slice,
                a_slice,
                &mut self.feedback_state,
                self.patch.feedback as i32,
                input_buffer,
                output_buffer,
            );

            i += call.n;
        }
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new(Patch::default(), 44100.0)
    }
}
