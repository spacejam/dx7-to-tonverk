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

//! Multi-segment envelope generator
//!
//! Implements DX7-style envelopes with quirks like:
//! - Vaguely logarithmic shape for ascending segments
//! - Direct jump above a threshold for ascending segments
//! - Specific logic and rates for plateaus

use super::dx_units::{operator_envelope_increment, operator_level, pitch_envelope_level};

/// Sentinel value indicating to use the previous level
const PREVIOUS_LEVEL: f32 = -100.0;

/// Generic multi-segment envelope
#[derive(Copy, Clone)]
pub struct Envelope<const NUM_STAGES: usize, const RESHAPE_ASCENDING: bool> {
    stage: usize,
    phase: f32,
    start: f32,
    increment: [f32; NUM_STAGES],
    level: [f32; NUM_STAGES],
    scale: f32,
}

impl<const NUM_STAGES: usize, const RESHAPE_ASCENDING: bool>
    Envelope<NUM_STAGES, RESHAPE_ASCENDING>
{
    /// Creates a new envelope
    pub fn new() -> Self {
        let mut env = Self {
            stage: NUM_STAGES - 1,
            phase: 1.0,
            start: 0.0,
            increment: [0.001; NUM_STAGES],
            level: [0.0; NUM_STAGES],
            scale: 1.0,
        };

        // Initialize levels with decreasing defaults
        for i in 0..NUM_STAGES {
            env.level[i] = 1.0 / (1 << i) as f32;
        }
        env.level[NUM_STAGES - 1] = 0.0;

        env
    }

    /// Initializes the envelope with a scale factor
    pub fn init(&mut self, scale: f32) {
        self.scale = scale;
        self.stage = NUM_STAGES - 1;
        self.phase = 1.0;
        self.start = 0.0;
        for i in 0..NUM_STAGES {
            self.increment[i] = 0.001;
            self.level[i] = 1.0 / (1 << i) as f32;
        }
        self.level[NUM_STAGES - 1] = 0.0;
    }

    /// Directly sets increment and level arrays
    pub fn set(&mut self, increment: &[f32; NUM_STAGES], level: &[f32; NUM_STAGES]) {
        self.increment.copy_from_slice(increment);
        self.level.copy_from_slice(level);
    }

    /// Renders envelope at a specific time (for "envelope scrubbing")
    pub fn render_at_sample(&self, t: f32, gate_duration: f32) -> f32 {
        if t > gate_duration {
            // In release phase
            let phase = (t - gate_duration) * self.increment[NUM_STAGES - 1];
            return if phase >= 1.0 {
                self.level[NUM_STAGES - 1]
            } else {
                let sustain_value = self.render_at_sample(gate_duration, gate_duration);
                self.value_at(NUM_STAGES - 1, phase, sustain_value)
            };
        }

        // Find which stage we're in
        let mut stage = 0;
        let mut remaining_time = t;
        for i in 0..(NUM_STAGES - 1) {
            let stage_duration = 1.0 / self.increment[i];
            if remaining_time < stage_duration {
                stage = i;
                break;
            }
            remaining_time -= stage_duration;
            stage = i + 1;
        }

        if stage == NUM_STAGES - 1 {
            remaining_time -= gate_duration;
            if remaining_time <= 0.0 {
                return self.level[NUM_STAGES - 2];
            } else if remaining_time * self.increment[NUM_STAGES - 1] > 1.0 {
                return self.level[NUM_STAGES - 1];
            }
        }

        self.value_at(stage, remaining_time * self.increment[stage], PREVIOUS_LEVEL)
    }

    /// Renders one sample of the envelope
    pub fn render(&mut self, gate: bool) -> f32 {
        self.render_scaled(gate, 1.0, 1.0, 1.0)
    }

    /// Renders one sample with rate and level scaling
    pub fn render_scaled(
        &mut self,
        gate: bool,
        rate: f32,
        ad_scale: f32,
        release_scale: f32,
    ) -> f32 {
        if gate {
            if self.stage == NUM_STAGES - 1 {
                // Trigger: move to attack stage
                self.start = self.value();
                self.stage = 0;
                self.phase = 0.0;
            }
        } else {
            if self.stage != NUM_STAGES - 1 {
                // Release: move to release stage
                self.start = self.value();
                self.stage = NUM_STAGES - 1;
                self.phase = 0.0;
            }
        }

        let scale_factor = if self.stage == NUM_STAGES - 1 {
            release_scale
        } else {
            ad_scale
        };
        self.phase += self.increment[self.stage] * rate * scale_factor;

        if self.phase >= 1.0 {
            if self.stage >= NUM_STAGES - 2 {
                // Stay in sustain or release
                self.phase = 1.0;
            } else {
                // Move to next stage
                self.phase = 0.0;
                self.stage += 1;
            }
            self.start = PREVIOUS_LEVEL;
        }

        self.value()
    }

    /// Calculates current envelope value
    #[inline]
    fn value(&self) -> f32 {
        self.value_at(self.stage, self.phase, self.start)
    }

    /// Calculates envelope value at a specific stage and phase
    #[inline]
    fn value_at(&self, stage: usize, mut phase: f32, start_level: f32) -> f32 {
        let mut from = if start_level == PREVIOUS_LEVEL {
            self.level[(stage + NUM_STAGES - 1) % NUM_STAGES]
        } else {
            start_level
        };
        let mut to = self.level[stage];

        if RESHAPE_ASCENDING && from < to {
            from = from.max(6.7);
            to = to.max(6.7);
            phase *= (2.5 - phase) * 0.666667;
        }

        phase * (to - from) + from
    }
}

impl<const NUM_STAGES: usize, const RESHAPE_ASCENDING: bool> Default
    for Envelope<NUM_STAGES, RESHAPE_ASCENDING>
{
    fn default() -> Self {
        Self::new()
    }
}

/// Operator envelope with DX7-specific quirks (4 stages, reshaped ascending)
#[derive(Copy, Clone)]
pub struct OperatorEnvelope {
    envelope: Envelope<4, true>,
}

impl OperatorEnvelope {
    /// Creates a new operator envelope
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),
        }
    }

    /// Initializes the envelope
    pub fn init(&mut self, scale: f32) {
        self.envelope.init(scale);
    }

    /// Configures the envelope from DX7 patch data
    pub fn set(&mut self, rate: &[u8; 4], level: &[u8; 4], global_level: u8) {
        // Configure levels
        for i in 0..4 {
            let mut level_scaled = operator_level(level[i] as i32);
            level_scaled = (level_scaled & !1) + global_level as i32 - 133;
            self.envelope.level[i] =
                0.125 * if level_scaled < 1 { 0.5 } else { level_scaled as f32 };
        }

        // Configure increments with DX7 quirks
        for i in 0..4 {
            let mut increment = operator_envelope_increment(rate[i] as i32);
            let from = self.envelope.level[(i + 4 - 1) % 4];
            let to = self.envelope.level[i];

            if from == to {
                // Quirk: for plateaus, the increment is scaled
                increment *= 0.6;
                if i == 0 && level[i] == 0 {
                    // Quirk: the attack plateau is faster
                    increment *= 20.0;
                }
            } else if from < to {
                let from_clamped = from.max(6.7);
                let to_clamped = to.max(6.7);
                if from_clamped == to_clamped {
                    // Quirk: because of the jump, the attack might disappear
                    increment = 1.0;
                } else {
                    // Quirk: because of the weird shape, the rate is adjusted
                    increment *= 7.2 / (to_clamped - from_clamped);
                }
            } else {
                increment *= 1.0 / (from - to);
            }
            self.envelope.increment[i] = increment * self.envelope.scale;
        }
    }

    /// Renders one sample
    pub fn render(&mut self, gate: bool) -> f32 {
        self.envelope.render(gate)
    }

    /// Renders one sample with scaling
    pub fn render_scaled(
        &mut self,
        gate: bool,
        rate: f32,
        ad_scale: f32,
        release_scale: f32,
    ) -> f32 {
        self.envelope
            .render_scaled(gate, rate, ad_scale, release_scale)
    }

    /// Renders at a specific sample time (for envelope scrubbing)
    pub fn render_at_sample(&self, t: f32, gate_duration: f32) -> f32 {
        self.envelope.render_at_sample(t, gate_duration)
    }
}

impl Default for OperatorEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

/// Pitch envelope (4 stages, no reshaping)
pub struct PitchEnvelope {
    envelope: Envelope<4, false>,
}

impl PitchEnvelope {
    /// Creates a new pitch envelope
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),
        }
    }

    /// Initializes the envelope
    pub fn init(&mut self, scale: f32) {
        self.envelope.init(scale);
    }

    /// Configures the envelope from DX7 patch data
    pub fn set(&mut self, rate: &[u8; 4], level: &[u8; 4]) {
        // Configure levels
        for i in 0..4 {
            self.envelope.level[i] = pitch_envelope_level(level[i] as i32);
        }

        // Configure increments
        for i in 0..4 {
            let from = self.envelope.level[(i + 4 - 1) % 4];
            let to = self.envelope.level[i];
            let mut increment = super::dx_units::pitch_envelope_increment(rate[i] as i32);

            if from != to {
                increment *= 1.0 / (from - to).abs();
            } else if i != 3 {
                increment = 0.2;
            }
            self.envelope.increment[i] = increment * self.envelope.scale;
        }
    }

    /// Renders one sample
    pub fn render(&mut self, gate: bool) -> f32 {
        self.envelope.render(gate)
    }

    /// Renders one sample with scaling
    pub fn render_scaled(
        &mut self,
        gate: bool,
        rate: f32,
        ad_scale: f32,
        release_scale: f32,
    ) -> f32 {
        self.envelope
            .render_scaled(gate, rate, ad_scale, release_scale)
    }

    /// Renders at a specific sample time (for envelope scrubbing)
    pub fn render_at_sample(&self, t: f32, gate_duration: f32) -> f32 {
        self.envelope.render_at_sample(t, gate_duration)
    }
}

impl Default for PitchEnvelope {
    fn default() -> Self {
        Self::new()
    }
}