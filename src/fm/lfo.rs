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

//! DX7-compatible LFO

use crate::fm::dx_units::{lfo_delay, lfo_frequency, pitch_mod_sensitivity};
use crate::fm::patch::ModulationParameters;
use crate::stmlib::dsp::sine;
use crate::stmlib::random::Random;

/// LFO waveform types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    /// Triangle wave (0)
    Triangle = 0,
    /// Ramp down / sawtooth wave (1)
    RampDown = 1,
    /// Ramp up / reverse sawtooth wave (2)
    RampUp = 2,
    /// Square wave (3)
    Square = 3,
    /// Sine wave (4)
    Sine = 4,
    /// Sample and hold / random stepped values (5)
    SAndH = 5,
}

impl From<u8> for Waveform {
    fn from(value: u8) -> Self {
        match value {
            1 => Waveform::RampDown,
            2 => Waveform::RampUp,
            3 => Waveform::Square,
            4 => Waveform::Sine,
            5 => Waveform::SAndH,
            _ => Waveform::Triangle,
        }
    }
}

/// DX7-style LFO
pub struct Lfo {
    phase: f32,
    frequency: f32,
    delay_phase: f32,
    delay_increment: [f32; 2],
    value: f32,
    random_value: f32,
    one_hz: f32,
    amp_mod_depth: f32,
    pitch_mod_depth: f32,
    waveform: Waveform,
    reset_phase: bool,
    phase_integral: i32,
}

impl Lfo {
    /// Creates a new LFO
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            frequency: 0.1,
            delay_phase: 0.0,
            delay_increment: [0.1, 0.1],
            value: 0.0,
            random_value: 0.0,
            one_hz: 0.0,
            amp_mod_depth: 0.0,
            pitch_mod_depth: 0.0,
            waveform: Waveform::Triangle,
            reset_phase: false,
            phase_integral: 0,
        }
    }

    /// Initializes the LFO
    pub fn init(&mut self, sample_rate: f32) {
        self.phase = 0.0;
        self.frequency = 0.1;
        self.delay_phase = 0.0;
        self.delay_increment[0] = 0.1;
        self.delay_increment[1] = 0.1;
        self.random_value = 0.0;
        self.value = 0.0;

        self.one_hz = 1.0 / sample_rate;

        self.amp_mod_depth = 0.0;
        self.pitch_mod_depth = 0.0;

        self.waveform = Waveform::Triangle;
        self.reset_phase = false;

        self.phase_integral = 0;
    }

    /// Configures the LFO from patch parameters
    pub fn set(&mut self, modulations: &ModulationParameters) {
        self.frequency = lfo_frequency(modulations.rate as i32) * self.one_hz;

        self.delay_increment = lfo_delay(modulations.delay as i32);
        self.delay_increment[0] *= self.one_hz;
        self.delay_increment[1] *= self.one_hz;

        self.waveform = Waveform::from(modulations.waveform);
        self.reset_phase = modulations.reset_phase != 0;

        self.amp_mod_depth = modulations.amp_mod_depth as f32 * 0.01;

        self.pitch_mod_depth = modulations.pitch_mod_depth as f32 * 0.01
            * pitch_mod_sensitivity(modulations.pitch_mod_sensitivity as i32);
    }

    /// Resets the LFO phase
    pub fn reset(&mut self) {
        if self.reset_phase {
            self.phase = 0.0;
        }
        self.delay_phase = 0.0;
    }

    /// Advances the LFO by one step (scaled)
    pub fn step(&mut self, scale: f32) {
        self.phase += scale * self.frequency;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.random_value = Random::get_float();
        }
        self.value = self.value();

        self.delay_phase += scale
            * self.delay_increment[if self.delay_phase < 0.5 { 0 } else { 1 }];
        if self.delay_phase >= 1.0 {
            self.delay_phase = 1.0;
        }
    }

    /// Scrubs the LFO to a specific sample position (for envelope scrubbing)
    pub fn scrub(&mut self, sample: f32) {
        let phase = sample * self.frequency;
        let phase_integral = phase as i32;
        let phase_fractional = phase - phase_integral as f32;
        self.phase = phase_fractional;
        if phase_integral != self.phase_integral {
            self.phase_integral = phase_integral;
            self.random_value = Random::get_float();
        }
        self.value = self.value();

        self.delay_phase = sample * self.delay_increment[0];
        if self.delay_phase > 0.5 {
            let sample = sample - 0.5 / self.delay_increment[0];
            self.delay_phase = 0.5 + sample * self.delay_increment[1];
            if self.delay_phase >= 1.0 {
                self.delay_phase = 1.0;
            }
        }
    }

    /// Calculates the current LFO value based on the waveform
    #[inline]
    fn value(&self) -> f32 {
        match self.waveform {
            Waveform::Triangle => {
                2.0 * if self.phase < 0.5 {
                    0.5 - self.phase
                } else {
                    self.phase - 0.5
                }
            }
            Waveform::RampDown => 1.0 - self.phase,
            Waveform::RampUp => self.phase,
            Waveform::Square => {
                if self.phase < 0.5 {
                    0.0
                } else {
                    1.0
                }
            }
            Waveform::Sine => 0.5 + 0.5 * sine(self.phase + 0.5),
            Waveform::SAndH => self.random_value,
        }
    }

    /// Returns the delay ramp value
    #[inline]
    pub fn delay_ramp(&self) -> f32 {
        if self.delay_phase < 0.5 {
            0.0
        } else {
            (self.delay_phase - 0.5) * 2.0
        }
    }

    /// Returns the pitch modulation amount
    #[inline]
    pub fn pitch_mod(&self) -> f32 {
        (self.value - 0.5) * self.delay_ramp() * self.pitch_mod_depth
    }

    /// Returns the amplitude modulation amount
    #[inline]
    pub fn amp_mod(&self) -> f32 {
        (1.0 - self.value) * self.delay_ramp() * self.amp_mod_depth
    }
}

impl Default for Lfo {
    fn default() -> Self {
        Self::new()
    }
}