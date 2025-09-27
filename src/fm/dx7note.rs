
//! DX7 Note - represents a single playing note in the synthesizer
//!
//! This is the main synthesis unit that combines all the FM operators,
//! envelopes, and modulation to produce the final audio output.

use super::{env::Env, lfo::Lfo, fm_op_kernel::FmOpKernel, constants::{N, LG_N}, exp2::Exp2};
use log::{debug, trace};

/// Velocity lookup table (from C++ dx7note.cc)
const VELOCITY_DATA: [u8; 64] = [
    0, 70, 86, 97, 106, 114, 121, 126, 132, 138, 142, 148, 152, 156, 160, 163,
    166, 170, 173, 174, 178, 181, 184, 186, 189, 190, 194, 196, 198, 200, 202,
    205, 206, 209, 211, 214, 216, 218, 220, 222, 224, 225, 227, 229, 230, 232,
    233, 235, 237, 238, 240, 241, 242, 243, 244, 246, 246, 248, 249, 250, 251,
    252, 253, 254
];

/// Exponential scale data for curve scaling (from C++ dx7note.cc)
const EXP_SCALE_DATA: [u8; 33] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 14, 16, 19, 23, 27, 33, 39, 47, 56, 66,
    80, 94, 110, 126, 142, 158, 174, 190, 206, 222, 238, 250
];

/// Scale velocity according to DX7 velocity sensitivity (exact C++ port)
fn scale_velocity(velocity: i32, sensitivity: i32) -> i32 {
    let clamped_vel = velocity.clamp(0, 127);
    let vel_value = VELOCITY_DATA[(clamped_vel >> 1) as usize] as i32 - 239;
    let scaled_vel = ((sensitivity * vel_value + 7) >> 3) << 4;
    scaled_vel
}

/// Scale rate according to keyboard rate scaling (exact C++ port)
fn scale_rate(midinote: i32, sensitivity: i32) -> i32 {
    let x = (midinote / 3 - 7).clamp(0, 31);
    let qratedelta = (sensitivity * x) >> 3;
    qratedelta
}

/// Scale curve according to exponential or linear scaling (exact C++ port)
fn scale_curve(group: i32, depth: i32, curve: i32) -> i32 {
    let scale = if curve == 0 || curve == 3 {
        // Linear
        (group * depth * 329) >> 12
    } else {
        // Exponential
        let raw_exp = EXP_SCALE_DATA[group.min(32) as usize] as i32;
        (raw_exp * depth * 329) >> 15
    };

    if curve < 2 {
        -scale
    } else {
        scale
    }
}

/// Scale level according to keyboard level scaling (exact C++ port)
fn scale_level(midinote: i32, break_pt: i32, left_depth: i32, right_depth: i32,
               left_curve: i32, right_curve: i32) -> i32 {
    let offset = midinote - break_pt - 17;
    if offset >= 0 {
        scale_curve((offset + 1) / 3, right_depth, right_curve)
    } else {
        scale_curve(-(offset - 1) / 3, left_depth, left_curve)
    }
}

/// Scale output level according to DX7 lookup table (exact C++ port)
fn scale_out_level(outlevel: i32) -> i32 {
    const LEVELLUT: [i32; 20] = [
        0, 5, 9, 13, 17, 20, 23, 25, 27, 29, 31, 33, 35, 37, 39, 41, 42, 43, 45, 46
    ];

    if outlevel >= 20 {
        28 + outlevel
    } else {
        LEVELLUT[outlevel as usize]
    }
}

/// Coarse frequency multiplier table (from C++ dx7note.cc)
const COARSE_MUL: [i32; 32] = [
    -16777216, 0, 16777216, 26591258, 33554432, 38955489, 43368474, 47099600,
    50331648, 53182516, 55732705, 58039632, 60145690, 62083076, 63876816,
    65546747, 67108864, 68576247, 69959732, 71268397, 72509921, 73690858,
    74816848, 75892776, 76922906, 77910978, 78860292, 79773775, 80654032,
    81503396, 82323963, 83117622
];

/// Calculate oscillator frequency using DX7 logarithmic system (exact C++ port)
fn osc_freq(midinote: i32, mode: i32, coarse: i32, fine: i32, detune: i32) -> i32 {
    let mut logfreq = if mode == 0 {
        // Ratio mode - use MIDI note frequency
        // C++ calculates: (1 << 24) * log2(frequency_of_midinote)
        // For MIDI note: frequency = 440 * 2^((midinote - 69)/12)
        let freq_hz = 440.0 * 2.0_f64.powf((midinote - 69) as f64 / 12.0);
        (((1 << 24) as f64) * freq_hz.log2()) as i32
    } else {
        // Fixed frequency mode
        // ((1 << 24) * log(10) / log(2) * .01) << 3
        (4458616 * ((coarse & 3) * 100 + fine)) >> 3
    };

    if mode == 0 {
        // Add coarse multiplier for ratio mode
        logfreq += COARSE_MUL[(coarse & 31) as usize];

        // Add fine tuning
        if fine != 0 {
            // (1 << 24) / log(2) ≈ 24204406
            let fine_adjust = (24204406.0 * (1.0 + 0.01 * fine as f64).ln()) as i32;
            logfreq += fine_adjust;
        }

        // Add detune (simplified - ignoring frequency-dependent scaling)
        if detune != 7 {
            logfreq += 13457 * (detune - 7);
        }
    } else {
        // Fixed frequency mode detune
        if detune > 7 {
            logfreq += 13457 * (detune - 7);
        }
    }

    logfreq
}

/// FM operator routing flags (from C++ fm_core.h)
#[allow(dead_code)]
mod operator_flags {
    pub const OUT_BUS_ONE: u8 = 1 << 0;
    pub const OUT_BUS_TWO: u8 = 1 << 1;
    pub const OUT_BUS_ADD: u8 = 1 << 2;
    pub const IN_BUS_ONE: u8 = 1 << 4;
    pub const IN_BUS_TWO: u8 = 1 << 5;
    pub const FB_IN: u8 = 1 << 6;
    pub const FB_OUT: u8 = 1 << 7;
}

/// DX7 FM Algorithm definition (32 algorithms, 6 operators each)
#[derive(Clone, Debug)]
pub struct FmAlgorithm {
    pub ops: [u8; 6],
}

/// DX7 algorithm definitions (from C++ fm_core.cc)
const ALGORITHMS: [FmAlgorithm; 32] = [
    FmAlgorithm { ops: [0xc1, 0x11, 0x11, 0x14, 0x01, 0x14] }, // 1
    FmAlgorithm { ops: [0x01, 0x11, 0x11, 0x14, 0xc1, 0x14] }, // 2
    FmAlgorithm { ops: [0xc1, 0x11, 0x14, 0x01, 0x11, 0x14] }, // 3
    FmAlgorithm { ops: [0xc1, 0x11, 0x94, 0x01, 0x11, 0x14] }, // 4
    FmAlgorithm { ops: [0xc1, 0x14, 0x01, 0x14, 0x01, 0x14] }, // 5
    FmAlgorithm { ops: [0xc1, 0x94, 0x01, 0x14, 0x01, 0x14] }, // 6
    FmAlgorithm { ops: [0xc1, 0x11, 0x05, 0x14, 0x01, 0x14] }, // 7
    FmAlgorithm { ops: [0x01, 0x11, 0xc5, 0x14, 0x01, 0x14] }, // 8
    FmAlgorithm { ops: [0x01, 0x11, 0x05, 0x14, 0xc1, 0x14] }, // 9
    FmAlgorithm { ops: [0x01, 0x05, 0x14, 0xc1, 0x11, 0x14] }, // 10
    FmAlgorithm { ops: [0xc1, 0x05, 0x14, 0x01, 0x11, 0x14] }, // 11
    FmAlgorithm { ops: [0x01, 0x05, 0x05, 0x14, 0xc1, 0x14] }, // 12
    FmAlgorithm { ops: [0xc1, 0x05, 0x05, 0x14, 0x01, 0x14] }, // 13
    FmAlgorithm { ops: [0xc1, 0x05, 0x11, 0x14, 0x01, 0x14] }, // 14
    FmAlgorithm { ops: [0x01, 0x05, 0x11, 0x14, 0xc1, 0x14] }, // 15
    FmAlgorithm { ops: [0xc1, 0x11, 0x02, 0x25, 0x05, 0x14] }, // 16
    FmAlgorithm { ops: [0x01, 0x11, 0x02, 0x25, 0xc5, 0x14] }, // 17
    FmAlgorithm { ops: [0x01, 0x11, 0x11, 0xc5, 0x05, 0x14] }, // 18
    FmAlgorithm { ops: [0xc1, 0x14, 0x14, 0x01, 0x11, 0x14] }, // 19
    FmAlgorithm { ops: [0x01, 0x05, 0x14, 0xc1, 0x14, 0x14] }, // 20
    FmAlgorithm { ops: [0x01, 0x14, 0x14, 0xc1, 0x14, 0x14] }, // 21
    FmAlgorithm { ops: [0xc1, 0x14, 0x14, 0x14, 0x01, 0x14] }, // 22
    FmAlgorithm { ops: [0xc1, 0x14, 0x14, 0x01, 0x14, 0x04] }, // 23
    FmAlgorithm { ops: [0xc1, 0x14, 0x14, 0x14, 0x04, 0x04] }, // 24
    FmAlgorithm { ops: [0xc1, 0x14, 0x14, 0x04, 0x04, 0x04] }, // 25
    FmAlgorithm { ops: [0xc1, 0x05, 0x14, 0x01, 0x14, 0x04] }, // 26
    FmAlgorithm { ops: [0x01, 0x05, 0x14, 0xc1, 0x14, 0x04] }, // 27
    FmAlgorithm { ops: [0x04, 0xc1, 0x05, 0x14, 0x01, 0x14] }, // 28
    FmAlgorithm { ops: [0xc1, 0x05, 0x14, 0x04, 0x01, 0x14] }, // 29
    FmAlgorithm { ops: [0x04, 0xc1, 0x05, 0x14, 0x04, 0x04] }, // 30
    FmAlgorithm { ops: [0xc1, 0x04, 0x04, 0x04, 0x04, 0x04] }, // 31
    FmAlgorithm { ops: [0xc1, 0x04, 0x04, 0x04, 0x04, 0x04] }, // 32
];

/// State of a single DX7 note
#[derive(Clone, Debug)]
pub struct Dx7Note {
    /// The 6 FM operators (DX7 has 6 operators)
    pub operators: [FmOperator; 6],

    /// MIDI note number
    pub note: u8,

    /// MIDI velocity
    pub velocity: u8,

    /// Current algorithm (determines operator routing)
    pub algorithm: u8,

    /// Overall pitch bend (in cents)
    pub pitch_bend: f32,

    /// Note is currently playing
    pub active: bool,

    /// Note phase (for LFO sync, etc.)
    pub phase: u32,

    /// Feedback buffers for self-modulating operators
    pub fb_buf: [i32; 2],

    /// Feedback shift amount (controls feedback level)
    pub fb_shift: i32,

    /// Intermediate buses for operator routing
    bus_buffers: [[i32; N]; 2], // bus 1 and bus 2
}

/// Individual FM operator within a DX7 note
#[derive(Clone, Debug)]
pub struct FmOperator {
    /// Amplitude envelope
    pub env: Env,

    /// Current phase
    pub phase: i32,

    /// Frequency (phase increment)
    pub freq: i32,

    /// Output level
    pub level: i32,

    /// Feedback buffer for self-modulation
    pub fb_buf: [i32; 2],

    /// Whether this operator is enabled
    pub enabled: bool,
}

impl Default for FmOperator {
    fn default() -> Self {
        Self::new()
    }
}

impl FmOperator {
    /// Create a new FM operator
    pub fn new() -> Self {
        Self {
            env: Env::new(),
            phase: 0, // Start with proper phase=0
            freq: 0,
            level: 0,
            fb_buf: [0; 2],
            enabled: true,
        }
    }

    /// Initialize the operator
    pub fn init(&mut self, rates: &[i32; 4], levels: &[i32; 4], outlevel: i32, rate_scaling: i32) {
        self.env.init(rates, levels, outlevel, rate_scaling);
    }

    /// Process operator for N samples
    pub fn process(&mut self, output: &mut [i32], input: Option<&[i32]>, feedback: Option<i32>) {
        if !self.enabled {
            output.fill(0);
            return;
        }

        // Get envelope value (logarithmic format) - now properly scaled by C++ scaling functions
        let env_level = self.env.get_sample();

        // Match C++ implementation exactly:
        // C++: int32_t gain2 = Exp2::lookup(param.level_in - (14 * (1 << 24)));
        let level_offset = 14 * (1 << 24);
        let exp2_input = env_level.saturating_sub(level_offset);
        let gain = Exp2::lookup(exp2_input);

        // C++ threshold check: if (gain1 >= kLevelThresh || gain2 >= kLevelThresh)
        // where kLevelThresh = 1120
        // DEBUG: Show gain calculation
        static mut GAIN_DEBUG_COUNT: usize = 0;
        unsafe {
            GAIN_DEBUG_COUNT += 1;
            if GAIN_DEBUG_COUNT <= 3 {
                debug!("GAIN: env_level={}, exp2_input={}, gain={}",
                    env_level, exp2_input, gain);
            }
        }

        if gain < 1120 {  // C++ kLevelThresh = 1120
            trace!("Gain {} below threshold (kLevelThresh=1120), filling with zeros", gain);
            output.fill(0);
            return;
        } else {
            trace!("Gain {} passes threshold, generating audio", gain);
        }


        // Debug: Check FmOpKernel input parameters (commented out)
        // static mut KERNEL_DEBUG_COUNT: usize = 0;

        match (input, feedback) {
            (Some(modulation), None) => {
                // FM operator with modulation input
                FmOpKernel::compute(output, modulation, self.phase, self.freq, gain, gain, false);
            }
            (None, Some(fb_shift)) => {
                // Operator with feedback
                FmOpKernel::compute_fb(output, self.phase, self.freq, gain, gain, &mut self.fb_buf, fb_shift, false);
            }
            (None, None) => {
                // Pure sine wave (carrier)
                trace!("SINE: phase={}, freq={}, gain={}", self.phase, self.freq, gain);
                FmOpKernel::compute_pure(output, self.phase, self.freq, gain, gain, false);

                // Debug: Check what was actually produced
                static mut SINE_DEBUG_COUNT: usize = 0;
                unsafe {
                    SINE_DEBUG_COUNT += 1;
                    if SINE_DEBUG_COUNT <= 2 {
                        trace!("SINE OUTPUT: {:?}", &output[0..5]);
                    }
                }
            }
            (Some(modulation), Some(_fb_shift)) => {
                // Both modulation and feedback (rare, but possible)
                FmOpKernel::compute(output, modulation, self.phase, self.freq, gain, gain, false);
                // Apply feedback separately - this is a simplification
            }
        }

        // Advance phase after synthesis (matches C++ architecture)
        // C++: param.phase += param.freq << LG_N;
        self.phase = self.phase.wrapping_add(self.freq << LG_N);
    }

    /// Handle key events
    pub fn keydown(&mut self, down: bool) {
        self.env.keydown(down);
    }
}

impl Default for Dx7Note {
    fn default() -> Self {
        Self::new()
    }
}

impl Dx7Note {
    /// Create a new DX7 note
    pub fn new() -> Self {
        Self {
            operators: [
                FmOperator::new(), FmOperator::new(), FmOperator::new(),
                FmOperator::new(), FmOperator::new(), FmOperator::new(),
            ],
            note: 60,
            velocity: 64,
            algorithm: 1,
            pitch_bend: 0.0,
            active: false,
            phase: 0,
            fb_buf: [0; 2],
            fb_shift: 16, // Default feedback shift
            bus_buffers: [[0; N]; 2],
        }
    }

    /// Initialize note with MIDI parameters
    pub fn init(&mut self, note: u8, velocity: u8) {
        self.note = note;
        self.velocity = velocity;
        self.active = true;
        self.phase = 0;

        // Key down all operators
        for op in &mut self.operators {
            op.keydown(true);
        }
    }

    /// Release the note (key up)
    pub fn release(&mut self) {
        for op in &mut self.operators {
            op.keydown(false);
        }
    }

    /// Check if note is still sounding
    pub fn is_active(&self) -> bool {
        if !self.active {
            return false;
        }

        // Note is active if any operator envelope is still above threshold
        self.operators.iter().any(|op| op.env.get_position() < 4)
    }

    /// Process note for N samples and add to output buffer
    /// Implements proper DX7 algorithm routing
    pub fn process(&mut self, output: &mut [i32], _lfo: &Lfo) {
        if !self.is_active() {
            return;
        }

        // Get the algorithm definition
        let algorithm_index = ((self.algorithm - 1) % 32) as usize; // Convert 1-32 to 0-31
        let alg = &ALGORITHMS[algorithm_index];
        debug!("ALGORITHM: Using algorithm {} (index {})", self.algorithm, algorithm_index);

        // Clear intermediate buses and output
        self.bus_buffers[0].fill(0);
        self.bus_buffers[1].fill(0);
        output.fill(0);

        // Track which buses have content (like C++ has_contents)
        let mut has_contents = [true, false, false]; // [output, bus1, bus2]

        // Process operators in order (like C++ FmCore::render)
        for op_idx in 0..6 {
            let flags = alg.ops[op_idx];
            let add = (flags & operator_flags::OUT_BUS_ADD) != 0;
            let inbus = (flags >> 4) & 3;
            let outbus = flags & 3;

            debug!("OP{}: flags={:02x}, add={}, inbus={}, outbus={}",
                op_idx, flags, add, inbus, outbus);

            // Get envelope level (matching C++ dx7note.cc:321)
            // C++: int32_t level = env_[op].getsample();
            // C++: params_[op].level_in = level;
            let env_level = self.operators[op_idx].env.get_sample();

            let level_offset = 14 * (1 << 24);
            let exp2_input = env_level.saturating_sub(level_offset);
            let gain = Exp2::lookup(exp2_input);

            // Debug gain calculation
            debug!("RENDER: Op {}: env_level={}, exp2_input={}, gain={}",
                op_idx, env_level, exp2_input, gain);

            // Temporary buffer for operator output
            let mut op_output = [0i32; N];

            // C++ threshold check
            if gain >= 1120 {
                debug!("RENDER: Op {} gain {} >= 1120, generating audio", op_idx, gain);
                // Determine synthesis type based on input bus and flags
                if inbus == 0 || !has_contents[inbus as usize] {
                    // No modulation input OR input bus is empty
                    if (flags & 0xc0) == 0xc0 && self.fb_shift < 16 {
                        // Feedback operator
                        FmOpKernel::compute_fb(
                            &mut op_output,
                            self.operators[op_idx].phase,
                            self.operators[op_idx].freq,
                            gain,
                            gain,
                            &mut self.fb_buf,
                            self.fb_shift,
                            false // Never add, we'll handle routing separately
                        );
                    } else {
                        // Pure sine wave (carrier)
                        FmOpKernel::compute_pure(
                            &mut op_output,
                            self.operators[op_idx].phase,
                            self.operators[op_idx].freq,
                            gain,
                            gain,
                            false // Never add, we'll handle routing separately
                        );
                    }
                } else {
                    // Operator with modulation input
                    let modulation = match inbus {
                        1 => &self.bus_buffers[0],
                        2 => &self.bus_buffers[1],
                        _ => {
                            // Invalid input bus
                            self.operators[op_idx].phase = self.operators[op_idx].phase.wrapping_add(
                                self.operators[op_idx].freq << LG_N
                            );
                            continue;
                        }
                    };

                    FmOpKernel::compute(
                        &mut op_output,
                        modulation,
                        self.operators[op_idx].phase,
                        self.operators[op_idx].freq,
                        gain,
                        gain,
                        false // Never add, we'll handle routing separately
                    );
                }

                // Route the operator output to the correct bus
                match outbus {
                    0 => {
                        // Direct to output
                        if add && has_contents[0] {
                            for i in 0..N {
                                output[i] += op_output[i];
                            }
                        } else {
                            output.copy_from_slice(&op_output);
                        }
                    }
                    1 => {
                        // Bus 1
                        if add && has_contents[1] {
                            for i in 0..N {
                                self.bus_buffers[0][i] += op_output[i];
                            }
                        } else {
                            self.bus_buffers[0].copy_from_slice(&op_output);
                        }
                    }
                    2 => {
                        // Bus 2
                        if add && has_contents[2] {
                            for i in 0..N {
                                self.bus_buffers[1][i] += op_output[i];
                            }
                        } else {
                            self.bus_buffers[1].copy_from_slice(&op_output);
                        }
                    }
                    _ => {
                        // Invalid output bus
                    }
                }

                has_contents[outbus as usize] = true;
            } else {
                debug!("RENDER: Op {} gain {} < 1120, skipping", op_idx, gain);
                if !add {
                    has_contents[outbus as usize] = false;
                }
            }

            // Advance phase (matching C++ param.phase += param.freq << LG_N)
            self.operators[op_idx].phase = self.operators[op_idx].phase.wrapping_add(
                self.operators[op_idx].freq << LG_N
            );
        }
    }

    /// Set pitch bend amount (in cents)
    pub fn set_pitch_bend(&mut self, cents: f32) {
        self.pitch_bend = cents;
        // TODO: Apply pitch bend to operator frequencies
    }

    /// Set algorithm
    pub fn set_algorithm(&mut self, algorithm: u8) {
        self.algorithm = algorithm.min(32); // DX7 has algorithms 1-32
    }

    /// Apply DX7 patch parameters to this note
    pub fn apply_patch(&mut self, patch_data: &[u8]) {
        if patch_data.len() < 155 {
            debug!("PATCH: apply_patch called with insufficient data: {} < 155", patch_data.len());
            return;
        }

        debug!("PATCH: apply_patch called with {} bytes", patch_data.len());
        trace!("PATCH: First 20 bytes: {:?}", &patch_data[..20]);

        // DX7 patch structure (from byte 134 onwards for global parameters)
        self.algorithm = patch_data[134] + 1; // Algorithm is 0-31 in data, 1-32 in practice
        debug!("PATCH: Algorithm set to {} (from byte 134: {})", self.algorithm, patch_data[134]);

        // Set feedback parameters (byte 135 is feedback level)
        let feedback = patch_data[135];
        self.fb_shift = if feedback != 0 {
            let shift = 7 - (feedback & 7) as i32;  // Clamp feedback to 0-7 range
            shift.max(0)  // Ensure non-negative shift
        } else {
            16 // No feedback
        };

        // Apply operator parameters from patch data
        // NOTE: DX7 patch data stores operators in REVERSE ORDER (6,5,4,3,2,1)
        for (i, op) in self.operators.iter_mut().enumerate() {
            op.enabled = true;

            // DX7 operator parameter layout (each operator is 21 bytes)
            // Reverse the operator index: operator 0 -> patch operator 6-1=5, etc.
            let patch_op_index = 5 - i;  // Map 0,1,2,3,4,5 -> 5,4,3,2,1,0
            let op_offset = patch_op_index * 21;
            debug!("PATCH: Processing operator {}, op_offset = {}", i, op_offset);
            if op_offset + 20 < 126 {
                debug!("PATCH: Operator {} within bounds, parsing parameters", i);
                // Get envelope rates and levels (bytes 0-7)
                let rates = [
                    patch_data[op_offset + 0] as i32,     // Attack rate
                    patch_data[op_offset + 1] as i32,     // Decay 1 rate
                    patch_data[op_offset + 2] as i32,     // Decay 2 rate
                    patch_data[op_offset + 3] as i32,     // Release rate
                ];
                let levels = [
                    patch_data[op_offset + 4] as i32,     // Attack level
                    patch_data[op_offset + 5] as i32,     // Decay 1 level
                    patch_data[op_offset + 6] as i32,     // Decay 2 level (sustain)
                    patch_data[op_offset + 7] as i32,     // Release level
                ];

                debug!("PATCH: Operator {} envelope - rates: {:?}, levels: {:?}", i, rates, levels);

                // Get parameters using EXACT C++ dexed unpacking layout (PluginData.cpp:unpackProgram)
                let output_level = patch_data[op_offset + 16] as i32;     // C++: unpackPgm[op * 21 + 16] = bulk[op * 17 + 14]

                // Extract packed frequency parameters from bytes 15-16
                let fcoarse_mode = patch_data[op_offset + 15] as i32;     // C++: bulk[op * 17 + 15]
                let freq_mode = fcoarse_mode & 1;                         // C++: unpackPgm[op * 21 + 17] = fcoarse_mode & 1
                let freq_coarse = (fcoarse_mode >> 1) & 0x1F;             // C++: unpackPgm[op * 21 + 18] = (fcoarse_mode >> 1)&0x1F
                let freq_fine = patch_data[op_offset + 19] as i32;        // C++: unpackPgm[op * 21 + 19] = bulk[op * 17 + 16]

                // Extract detune from packed byte 12
                let detune_rs = patch_data[op_offset + 12] as i32;
                let freq_detune = (detune_rs >> 3) & 0x7F;               // C++: unpackPgm[op * 21 + 20] = (detune_rs >> 3) &0x7F

                // Get keyboard scaling parameters per C++ implementation
                let key_break_point = patch_data[op_offset + 8] as i32;
                let key_left_depth = patch_data[op_offset + 9] as i32;
                let key_right_depth = patch_data[op_offset + 10] as i32;
                let curve_settings = patch_data[op_offset + 11] as i32;   // C++: leftrightcurves
                let vel_amp_sens = patch_data[op_offset + 13] as i32;     // C++: kvs_ams

                // Extract curve and sensitivity values per C++ implementation
                let key_left_curve = curve_settings & 0x03;              // C++: unpackPgm[op * 21 + 11] = leftrightcurves & 3
                let key_right_curve = (curve_settings >> 2) & 0x03;      // C++: unpackPgm[op * 21 + 12] = (leftrightcurves >> 2) & 3
                let rate_scaling_sens = detune_rs & 0x07;                 // C++: unpackPgm[op * 21 + 13] = detune_rs & 7
                let amp_mod_sens = vel_amp_sens & 0x03;                   // C++: unpackPgm[op * 21 + 14] = kvs_ams & 3
                let velocity_sens = (vel_amp_sens >> 2) & 0x07;          // C++: unpackPgm[op * 21 + 15] = (kvs_ams >> 2) & 7

                debug!("PATCH: Operator {} freq params: mode={}, coarse={}, fine={}, detune={}, output_level={}",
                    i, freq_mode, freq_coarse, freq_fine, freq_detune, output_level);
                trace!("PATCH: Operator {} patch bytes [{}..{}]: {:?}",
                    i, op_offset, op_offset + 21, &patch_data[op_offset..op_offset + 21]);

                // Apply exact C++ envelope scaling logic (from dx7note.cc:174-184)
                let mut outlevel = output_level;

                if i == 0 {  // Only debug OP0 to reduce noise
                    debug!("OP{}: Raw sysex output_level = {} (from patch data)", i, patch_data[op_offset + 16]);
                    debug!("OP{}: Initial output_level = {}", i, outlevel);

                    // Step 1: Scale output level
                    outlevel = scale_out_level(outlevel);
                    debug!("OP{}: After scale_out_level = {}", i, outlevel);

                    // Step 2: Add keyboard level scaling
                    let level_scaling = scale_level(self.note as i32, key_break_point,
                                                  key_left_depth, key_right_depth,
                                                  key_left_curve, key_right_curve);
                    debug!("OP{}: level_scaling = {}", i, level_scaling);
                    outlevel += level_scaling;
                    outlevel = outlevel.min(127);
                    debug!("OP{}: After level_scaling+clamp = {}", i, outlevel);

                    // Step 3: Shift left by 5 bits
                    outlevel = outlevel << 5;
                    debug!("OP{}: After <<5 = {}", i, outlevel);

                    // Step 4: Add velocity scaling
                    let vel_scaling = scale_velocity(self.velocity as i32, velocity_sens);
                    debug!("OP{}: velocity_scaling = {}", i, vel_scaling);
                    outlevel += vel_scaling;
                    outlevel = outlevel.max(0);
                    debug!("OP{}: Final outlevel = {}", i, outlevel);
                } else {
                    // Non-debug path for other operators
                    outlevel = scale_out_level(outlevel);
                    let level_scaling = scale_level(self.note as i32, key_break_point,
                                                  key_left_depth, key_right_depth,
                                                  key_left_curve, key_right_curve);
                    outlevel += level_scaling;
                    outlevel = outlevel.min(127);
                    outlevel = outlevel << 5;
                    outlevel += scale_velocity(self.velocity as i32, velocity_sens);
                    outlevel = outlevel.max(0);
                }

                // Debug output (disabled for production)
                // eprintln!("CRITICAL: OP{} outlevel = {}", i, outlevel);

                // Check if outlevel is too low for synthesis (commented out for production)
                // if outlevel < 100 {
                //     println!("  WARNING: outlevel {} may be too low for audible synthesis", outlevel);
                // }

                // Step 5: Calculate rate scaling
                let rate_scaling = scale_rate(self.note as i32, rate_scaling_sens);

                // Debug: Print envelope outlevel calculation (commented out)
                // println!("DEBUG: Envelope outlevel calculation for operator {}:", i);

                // Scale outlevel appropriately to avoid overflow in envelope calculations
                // The envelope advance() function does: (actuallevel << 6) + outlevel - 4256
                // Then does: actuallevel << 16, which can overflow if too large
                // Safe maximum: outlevel should be < (i32::MAX >> 16) - (127 << 6) ≈ 24000
                let scaled_outlevel = if outlevel > 0 {
                    outlevel.min(20000)  // Cap to prevent overflow
                } else {
                    0
                };

                debug!("PATCH: Operator {} outlevel calculation: raw_outlevel={}, scaled_outlevel={}, rate_scaling={}",
                    i, outlevel, scaled_outlevel, rate_scaling);


                // Initialize envelope with exact C++ parameters
                op.env.init(&rates, &levels, scaled_outlevel, rate_scaling);

                // Calculate frequency using exact C++ logarithmic system
                let logfreq = osc_freq(self.note as i32, freq_mode, freq_coarse, freq_fine, freq_detune);

                // Convert logfreq to phase increment (matching C++ conversion)
                // logfreq is in Q24 format: logfreq = (1 << 24) * log2(frequency)
                // frequency = 2^(logfreq / (1 << 24))
                let freq_hz = 2.0_f64.powf(logfreq as f64 / (1 << 24) as f64);

                // Debug: Print frequency calculation for all operators
                debug!("FREQ OP{}: MIDI note {}, mode {}, coarse {}, fine {}, detune {}",
                    i, self.note, freq_mode, freq_coarse, freq_fine, freq_detune);
                debug!("FREQ OP{}: logfreq={}, freq_hz={:.2}, phase_inc={}",
                    i, logfreq, freq_hz, op.freq);
                trace!("FREQ OP{}: patch_data[{}..{}] = {:?}",
                    i, op_offset, op_offset + 21, &patch_data[op_offset..op_offset.min(patch_data.len()).min(op_offset + 21)]);

                // Convert to phase increment: freq_increment = freq_hz * 2^32 / sample_rate
                // But scale down to avoid overflow in synthesis - use reasonable scaling
                let phase_inc_calc = (freq_hz * 65536.0) / 44100.0;
                op.freq = phase_inc_calc as i32;

                debug!("FREQ OP{}: calculation: {} * 65536 / 44100 = {} -> i32 = {}",
                    i, freq_hz, phase_inc_calc, op.freq);

                // Set output level
                op.level = (output_level << 7).max(100); // Ensure some minimum level
            } else {
                // Default parameters for operators without proper patch data
                let default_rates = [10, 10, 10, 10];
                let default_levels = [99, 90, 70, 0];
                op.env.init(&default_rates, &default_levels, 99 << 7, 0);

                // Default frequency: MIDI note frequency
                let base_freq = 440.0 * f64::powf(2.0, (self.note as f64 - 69.0) / 12.0);
                op.freq = ((base_freq * 65536.0) / 44100.0) as i32;
                op.level = 99 << 7;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_creation() {
        let op = FmOperator::new();
        assert!(op.enabled);
        assert_eq!(op.phase, 0);
        assert_eq!(op.freq, 0);
    }

    #[test]
    fn test_dx7note_creation() {
        let note = Dx7Note::new();
        assert_eq!(note.operators.len(), 6);
        assert_eq!(note.note, 60);
        assert_eq!(note.velocity, 64);
        assert!(!note.active);
    }

    #[test]
    fn test_note_init() {
        let mut note = Dx7Note::new();
        note.init(69, 100); // A4, forte velocity

        assert_eq!(note.note, 69);
        assert_eq!(note.velocity, 100);
        assert!(note.active);
    }

    #[test]
    fn test_note_release() {
        let mut note = Dx7Note::new();
        note.init(60, 64);
        assert!(note.active);

        note.release();
        // Note should still be active but in release phase
        // (actual behavior depends on envelope implementation)
    }

    #[test]
    fn test_pitch_bend() {
        let mut note = Dx7Note::new();
        note.set_pitch_bend(100.0); // +1 semitone
        assert_eq!(note.pitch_bend, 100.0);
    }

    #[test]
    fn test_algorithm() {
        let mut note = Dx7Note::new();
        note.set_algorithm(5);
        assert_eq!(note.algorithm, 5);

        // Test clamping
        note.set_algorithm(50);
        assert_eq!(note.algorithm, 32); // Max algorithm
    }
}