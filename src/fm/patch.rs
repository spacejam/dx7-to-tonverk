// Copyright 2021 Emilie Gillet.
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

//! DX7 patch data structures

/// Size of SysEx patch data
pub const SYX_SIZE: usize = 128;

const BANK_PATCHES: usize = 32;

const HEADER_BANK: [u8; 6] = [0xF0, 0x43, 0x00, 0x09, 0x20, 0x00];
// const HEADER_SINGLE: [u8; 6] = [0xF0, 0x43, 0x00, 0x00, 0x01, 0x1B];

/// DX6 voice bank (32 voices = 32 * 128 bytes packed + 2 bytes checksum)
pub const BULK_FULL_SYSEX_SIZE: usize = 4104;

/// A bank of 32 dx7 patches parsed from sysex.
#[derive(Debug, Clone)]
pub struct PatchBank {
    /// The array of 32 patches.
    pub patches: [Patch; BANK_PATCHES],
}

impl PatchBank {
    pub fn new(data: &[u8]) -> PatchBank {
        assert_eq!(
            data.len(),
            BULK_FULL_SYSEX_SIZE,
            "currently only support parsing banks with exactly 32 patches, which must be {} bytes exactly",
            BULK_FULL_SYSEX_SIZE
        );
        assert_eq!(&data[..6], &HEADER_BANK[..6], "sysex header is not correct");

        let mut patches = [Patch::default(); BANK_PATCHES];

        let patch_data = &data[HEADER_BANK.len()..];

        for idx in 0..BANK_PATCHES {
            let start = idx * SYX_SIZE;
            let end = (idx + 1) * SYX_SIZE;
            let patch = Patch::new(&patch_data[start..end]);
            patches[idx] = patch;
        }

        PatchBank { patches }
    }
}

/// DX7 envelope parameters (4-stage)
#[derive(Debug, Clone, Copy, Default)]
pub struct Envelope {
    /// Rate for each of the 4 envelope stages
    pub rate: [u8; 4],
    /// Level for each of the 4 envelope stages
    pub level: [u8; 4],
}

/// Keyboard scaling parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardScaling {
    /// Break point key (0-99)
    pub break_point: u8,
    /// Depth of scaling on the left side of break point
    pub left_depth: u8,
    /// Depth of scaling on the right side of break point
    pub right_depth: u8,
    /// Curve type for left side (0-3)
    pub left_curve: u8,
    /// Curve type for right side (0-3)
    pub right_curve: u8,
}

/// DX7 operator parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct Operator {
    /// Amplitude envelope
    pub envelope: Envelope,
    /// Keyboard scaling settings
    pub keyboard_scaling: KeyboardScaling,
    /// Rate scaling (0-7)
    pub rate_scaling: u8,
    /// Amplitude modulation sensitivity (0-3)
    pub amp_mod_sensitivity: u8,
    /// Velocity sensitivity (0-7)
    pub velocity_sensitivity: u8,
    /// Output level (0-99)
    pub level: u8,
    /// Oscillator mode: 0 = ratio, 1 = fixed frequency
    pub mode: u8,
    /// Coarse frequency multiplier (0-31)
    pub coarse: u8,
    /// Fine frequency adjustment (0-99, multiplies frequency by 1 + 0.01 * fine)
    pub fine: u8,
    /// Detune amount (0-14)
    pub detune: u8,
}

/// LFO modulation parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct ModulationParameters {
    /// LFO rate (0-99)
    pub rate: u8,
    /// LFO delay (0-99)
    pub delay: u8,
    /// Pitch modulation depth (0-99)
    pub pitch_mod_depth: u8,
    /// Amplitude modulation depth (0-99)
    pub amp_mod_depth: u8,
    /// Reset phase on note trigger
    pub reset_phase: u8,
    /// LFO waveform (0-5)
    pub waveform: u8,
    /// Pitch modulation sensitivity
    pub pitch_mod_sensitivity: u8,
}

/// Complete DX7 patch
#[derive(Debug, Clone, Copy)]
pub struct Patch {
    /// Six operators (DX7 has 6 operators)
    pub op: [Operator; 6],
    /// Pitch envelope
    pub pitch_envelope: Envelope,
    /// Algorithm number (0-31)
    pub algorithm: u8,
    /// Feedback amount (0-7)
    pub feedback: u8,
    /// Reset oscillator phases on note trigger
    pub reset_phase: u8,
    /// LFO/modulation parameters
    pub modulations: ModulationParameters,
    /// Transpose value (0-48)
    pub transpose: u8,
    /// Patch name (10 characters)
    pub name: [char; 10],
    /// Active operators bitmask
    pub active_operators: u8,
}

impl Default for Patch {
    fn default() -> Self {
        Self {
            op: [Operator::default(); 6],
            pitch_envelope: Envelope::default(),
            algorithm: 0,
            feedback: 0,
            reset_phase: 0,
            modulations: ModulationParameters::default(),
            transpose: 0,
            name: [' '; 10],
            active_operators: 0x3f, // All 6 operators active
        }
    }
}

impl Patch {
    /// Creates a new patch from SYSEX bytes.
    pub fn new(data: &[u8]) -> Self {
        let mut ret = Self::default();
        ret.unpack(data);
        ret
    }

    /// Unpacks a DX7 SysEx patch from raw bytes
    fn unpack(&mut self, data: &[u8]) {
        assert_eq!(
            data.len(),
            SYX_SIZE,
            "Patch data not exactly {} bytes long",
            SYX_SIZE
        );

        // Unpack the 6 operators
        for i in 0..6 {
            let o = &mut self.op[i];
            let op_data = &data[i * 17..];

            // Envelope rates and levels
            for j in 0..4 {
                o.envelope.rate[j] = (op_data[j] & 0x7f).min(99);
                o.envelope.level[j] = (op_data[4 + j] & 0x7f).min(99);
            }

            // Keyboard scaling
            o.keyboard_scaling.break_point = (op_data[8] & 0x7f).min(99);
            o.keyboard_scaling.left_depth = (op_data[9] & 0x7f).min(99);
            o.keyboard_scaling.right_depth = (op_data[10] & 0x7f).min(99);
            o.keyboard_scaling.left_curve = op_data[11] & 0x3;
            o.keyboard_scaling.right_curve = (op_data[11] >> 2) & 0x3;

            // Other operator parameters
            o.rate_scaling = op_data[12] & 0x7;
            o.amp_mod_sensitivity = op_data[13] & 0x3;
            o.velocity_sensitivity = (op_data[13] >> 2) & 0x7;
            o.level = (op_data[14] & 0x7f).min(99);
            o.mode = op_data[15] & 0x1;
            o.coarse = (op_data[15] >> 1) & 0x1f;
            o.fine = (op_data[16] & 0x7f).min(99);
            o.detune = ((op_data[12] >> 3) & 0xf).min(14);
        }

        // Pitch envelope
        for j in 0..4 {
            self.pitch_envelope.rate[j] = (data[102 + j] & 0x7f).min(99);
            self.pitch_envelope.level[j] = (data[106 + j] & 0x7f).min(99);
        }

        // Global parameters
        self.algorithm = data[110] & 0x1f;
        self.feedback = data[111] & 0x7;
        self.reset_phase = (data[111] >> 3) & 0x1;

        // Modulation parameters
        self.modulations.rate = (data[112] & 0x7f).min(99);
        self.modulations.delay = (data[113] & 0x7f).min(99);
        self.modulations.pitch_mod_depth = (data[114] & 0x7f).min(99);
        self.modulations.amp_mod_depth = (data[115] & 0x7f).min(99);
        self.modulations.reset_phase = data[116] & 0x1;
        self.modulations.waveform = ((data[116] >> 1) & 0x7).min(5);
        self.modulations.pitch_mod_sensitivity = data[116] >> 4;

        self.transpose = (data[117] & 0x7f).min(48);

        // Patch name
        for i in 0..10 {
            self.name[i] = char::from(data[118 + i] & 0x7f);
        }

        self.active_operators = 0x3f; // All operators active by default
    }
}
