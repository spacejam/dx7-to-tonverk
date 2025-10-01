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

//! (mostly) Idiomatic Rust port of Mutable Instruments Plaits DX7/FM synthesis engine.
//!
//! This crate provides a port of the FM synthesis components from the
//! Mutable Instruments Plaits Eurorack module, focusing specifically on
//! the DX7-style FM synthesis engine.

#![warn(missing_docs)]

pub mod fm;
mod stmlib;

/// Sample rate used by the synthesis engine (in Hz)
pub const SAMPLE_RATE: f32 = 48000.0;

/// Maximum block size for audio processing
pub const MAX_BLOCK_SIZE: usize = 24;

/// Number of operators for DX7
const NUM_OPERATORS: usize = 6;

/// Number of algorithms for DX7;
const NUM_ALGORITHMS: usize = 32;

pub use fm::patch::{Patch, PatchBank};

use fm::lfo::Lfo;
use fm::voice::Parameters;
use fm::voice::Voice;

impl Patch {
    /// midi_note is based on midi note 60.0 correlating to C4 at 260hz. midi_note of 69.0 corresponds to
    /// A4 at 437hz.
    pub fn generate_samples(
        self,
        midi_note: f32,
        sample_rate: u32,
        duration: std::time::Duration,
    ) -> Vec<f32> {
        const MAX_BLOCK_SIZE: usize = 24; // Match C++ implementation
        let n_samples = duration.as_millis() as usize * (sample_rate as usize / 1000) as usize;
        let silence_threshold = 0.0001f32;
        let silence_duration_samples = (sample_rate as usize * 100) / 1000; // 100ms

        let mut voice = Voice::new(self.clone(), sample_rate as f32);
        let mut lfo = Lfo::new();
        lfo.init(sample_rate as f32);
        lfo.set(&self.modulations);
        lfo.reset();

        let mut output = Vec::new();

        // Phase 1: Render with gate on for the requested duration
        let mut parameters = Parameters {
            gate: true,
            sustain: false,
            velocity: 1.0,
            note: midi_note,
            ..Parameters::default()
        };

        let mut remaining = n_samples;
        while remaining > 0 {
            let block_size = remaining.min(MAX_BLOCK_SIZE);

            // Step the LFO
            lfo.step(block_size as f32);

            // Apply LFO modulations to parameters
            parameters.pitch_mod = lfo.pitch_mod();
            parameters.amp_mod = lfo.amp_mod();

            let mut buf = vec![0.0_f32; block_size * 3]; // render_temp needs 3x size
            voice.render_temp(&parameters, &mut buf);
            output.extend_from_slice(&buf[..block_size]);
            remaining -= block_size;
        }

        // Phase 2: Turn gate off and render until 100ms of silence
        parameters.gate = false;
        let mut consecutive_silent_samples = 0;

        loop {
            // Step the LFO
            lfo.step(MAX_BLOCK_SIZE as f32);

            // Apply LFO modulations to parameters
            parameters.pitch_mod = lfo.pitch_mod();
            parameters.amp_mod = lfo.amp_mod();

            let mut chunk = vec![0.0_f32; MAX_BLOCK_SIZE * 3];
            voice.render_temp(&parameters, &mut chunk);

            // Check for silence in the rendered output
            let rendered = &chunk[..MAX_BLOCK_SIZE];
            for &sample in rendered {
                if sample.abs() < silence_threshold {
                    consecutive_silent_samples += 1;
                } else {
                    consecutive_silent_samples = 0;
                }
            }

            output.extend_from_slice(rendered);

            // Check if we've accumulated enough silence
            if consecutive_silent_samples >= silence_duration_samples {
                // Truncate to end after the silence duration
                let truncate_to = output
                    .len()
                    .saturating_sub(consecutive_silent_samples - silence_duration_samples);
                output.truncate(truncate_to);
                return output;
            }

            // Safety limit: don't render more than 10 seconds total
            if output.len() > sample_rate as usize * 10 {
                break;
            }
        }

        output
    }
}
