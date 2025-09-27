
use anyhow::{anyhow, Result};
use std::fs;

/// DX7 SYSEX constants
const SYSEX_START: u8 = 0xF0;
const SYSEX_END: u8 = 0xF7;
const YAMAHA_ID: u8 = 0x43;
const DX7_SINGLE_VOICE: u8 = 0x00; // Single voice data
const DX7_32_VOICES: u8 = 0x09; // 32 voice bank

/// DX7 patch data (single voice = 155 bytes)
pub const DX7_VOICE_SIZE: usize = 155;

/// DX7 voice bank (32 voices = 32 * 128 bytes packed + 2 bytes checksum)
pub const DX7_BANK_SIZE: usize = 4096;

/// Parsed DX7 patch data
#[derive(Debug, Clone)]
pub struct Dx7Patch {
    /// 6 FM operators
    pub operators: [OperatorParams; 6],
    /// Global synthesis parameters
    pub global: GlobalParams,
    /// Patch name (10 characters)
    pub name: String,
}

impl Dx7Patch {
    /// Create a new patch with default parameters
    pub fn new(name: &str) -> Self {
        Self {
            operators: [OperatorParams::default(); 6],
            global: GlobalParams::default(),
            name: name.to_string(),
        }
    }

    /// Create a new patch from raw data
    pub fn from_data(data: &[u8]) -> Result<Self> {
        if data.len() < DX7_VOICE_SIZE {
            return Err(anyhow!("Voice data too short: {} bytes", data.len()));
        }

        let mut voice_data = [0u8; DX7_VOICE_SIZE];
        voice_data.copy_from_slice(&data[..DX7_VOICE_SIZE]);

        // Extract voice name (last 10 bytes of voice data)
        let name_bytes = &voice_data[145..155];
        let name = String::from_utf8_lossy(name_bytes)
            .trim_end_matches('\0')
            .trim()
            .to_string();

        // Parse operators
        let mut operators = [OperatorParams::default(); 6];
        for op in 0..6 {
            let base = op * 21;
            operators[op] = OperatorParams {
                rates: Eg {
                    attack: voice_data[base + 0],   // R1
                    decay1: voice_data[base + 1],   // R2
                    decay2: voice_data[base + 2],   // R3
                    release: voice_data[base + 3],  // R4
                },
                levels: Eg {
                    attack: voice_data[base + 4],   // L1
                    decay1: voice_data[base + 5],   // L2
                    decay2: voice_data[base + 6],   // L3
                    release: voice_data[base + 7],  // L4
                },
                level_scaling_bp: voice_data[base + 8],
                level_scaling_ld: voice_data[base + 9],
                level_scaling_rd: voice_data[base + 10],
                level_scaling_lc: voice_data[base + 11],
                level_scaling_rc: voice_data[base + 12],
                rate_scaling: voice_data[base + 13],
                amp_mod_sens: voice_data[base + 14],
                velocity_sens: voice_data[base + 15],
                output_level: voice_data[base + 16],
                osc_mode: voice_data[base + 17],
                coarse_freq: voice_data[base + 18],
                fine_freq: voice_data[base + 19],
                detune: voice_data[base + 20],
            };
        }

        // Parse global parameters
        let global = GlobalParams {
            pitch_eg_rate: [
                voice_data[126],
                voice_data[127],
                voice_data[128],
                voice_data[129],
            ],
            pitch_eg_level: [
                voice_data[130],
                voice_data[131],
                voice_data[132],
                voice_data[133],
            ],
            algorithm: voice_data[134],
            feedback: voice_data[135],
            osc_sync: voice_data[136],
            lfo_speed: voice_data[137],
            lfo_delay: voice_data[138],
            lfo_pitch_mod_depth: voice_data[139],
            lfo_amp_mod_depth: voice_data[140],
            lfo_sync: voice_data[141],
            lfo_waveform: voice_data[142],
            pitch_mod_sens: voice_data[143],
            transpose: voice_data[144],
        };

        Ok(Self {
            operators,
            global,
            name: if name.is_empty() {
                "INIT VOICE".to_string()
            } else {
                name
            },
        })
    }

    /// Get operator parameters for operator `op` (0-5)
    pub fn get_operator(&self, op: usize) -> Result<OperatorParams> {
        if op >= 6 {
            return Err(anyhow!("Invalid operator index: {}", op));
        }
        Ok(self.operators[op].clone())
    }

    /// Get global parameters
    pub fn get_global(&self) -> GlobalParams {
        self.global.clone()
    }

    /// Generate raw data array from structured members
    pub fn to_data(&self) -> [u8; DX7_VOICE_SIZE] {
        let mut data = [0u8; DX7_VOICE_SIZE];

        // Update operator data
        for op in 0..6 {
            let base = op * 21;
            let op_params = &self.operators[op];

            data[base + 0..base + 4].copy_from_slice(&op_params.rates.as_array());
            data[base + 4..base + 8].copy_from_slice(&op_params.levels.as_array());
            data[base + 8] = op_params.level_scaling_bp;
            data[base + 9] = op_params.level_scaling_ld;
            data[base + 10] = op_params.level_scaling_rd;

            // Pack left/right curves into byte 11 (DX7 format)
            let curve_settings = (op_params.level_scaling_lc & 0x03) | ((op_params.level_scaling_rc & 0x03) << 2);
            data[base + 11] = curve_settings;

            // Pack detune and rate scaling into byte 12 (DX7 format)
            let detune_rs = ((op_params.detune & 0x7F) << 3) | (op_params.rate_scaling & 0x07);
            data[base + 12] = detune_rs;

            // Byte 13 is used for vel_amp_sens in DX7 format (parsed as kvs_ams)
            let vel_amp_sens = ((op_params.velocity_sens & 0x07) << 2) | (op_params.amp_mod_sens & 0x03);
            data[base + 13] = vel_amp_sens;

            // Pack oscillator mode and coarse frequency into byte 15 (DX7 format)
            let fcoarse_mode = (op_params.osc_mode & 0x01) | ((op_params.coarse_freq & 0x1F) << 1);
            data[base + 15] = fcoarse_mode;

            data[base + 16] = op_params.output_level;
            data[base + 19] = op_params.fine_freq;
        }

        // Update global data
        data[126..130].copy_from_slice(&self.global.pitch_eg_rate);
        data[130..134].copy_from_slice(&self.global.pitch_eg_level);
        data[134] = self.global.algorithm;
        data[135] = self.global.feedback;
        data[136] = self.global.osc_sync;
        data[137] = self.global.lfo_speed;
        data[138] = self.global.lfo_delay;
        data[139] = self.global.lfo_pitch_mod_depth;
        data[140] = self.global.lfo_amp_mod_depth;
        data[141] = self.global.lfo_sync;
        data[142] = self.global.lfo_waveform;
        data[143] = self.global.pitch_mod_sens;
        data[144] = self.global.transpose;

        // Set patch name (10 bytes, padded with spaces)
        let name_bytes = format!("{:10}", self.name);
        data[145..155].copy_from_slice(name_bytes.as_bytes());

        data
    }
}

/// DX7 envelope generator (ADSR) parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct Eg {
    pub attack: u8,    // 0-99
    pub decay1: u8,    // 0-99
    pub decay2: u8,    // 0-99 (sustain rate)
    pub release: u8,   // 0-99
}

impl Eg {
    /// Get as array for compatibility with existing envelope code
    pub fn as_array(&self) -> [u8; 4] {
        [self.attack, self.decay1, self.decay2, self.release]
    }

    /// Set from array for compatibility
    pub fn from_array(values: [u8; 4]) -> Self {
        Self {
            attack: values[0],
            decay1: values[1],
            decay2: values[2],
            release: values[3],
        }
    }
}

/// DX7 operator parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct OperatorParams {
    pub rates: Eg,            // Envelope rates (ADSR)
    pub levels: Eg,           // Envelope levels (ADSR)
    pub level_scaling_bp: u8, // Level scaling break point
    pub level_scaling_ld: u8, // Left depth
    pub level_scaling_rd: u8, // Right depth
    pub level_scaling_lc: u8, // Left curve
    pub level_scaling_rc: u8, // Right curve
    pub rate_scaling: u8,     // Rate scaling
    pub amp_mod_sens: u8,     // Amplitude modulation sensitivity
    pub velocity_sens: u8,    // Velocity sensitivity
    pub output_level: u8,     // Output level
    pub osc_mode: u8,         // Oscillator mode
    pub coarse_freq: u8,      // Coarse frequency
    pub fine_freq: u8,        // Fine frequency
    pub detune: u8,           // Detune
}

/// DX7 global parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalParams {
    pub pitch_eg_rate: [u8; 4],  // Pitch envelope rates
    pub pitch_eg_level: [u8; 4], // Pitch envelope levels
    pub algorithm: u8,           // Algorithm (0-31)
    pub feedback: u8,            // Feedback (0-7)
    pub osc_sync: u8,            // Oscillator sync
    pub lfo_speed: u8,           // LFO speed
    pub lfo_delay: u8,           // LFO delay
    pub lfo_pitch_mod_depth: u8, // LFO pitch modulation depth
    pub lfo_amp_mod_depth: u8,   // LFO amplitude modulation depth
    pub lfo_sync: u8,            // LFO sync
    pub lfo_waveform: u8,        // LFO waveform
    pub pitch_mod_sens: u8,      // Pitch modulation sensitivity
    pub transpose: u8,           // Transpose
}

/// Parse a SYSEX file and extract DX7 patches
pub fn parse_sysex_file(filename: &str) -> Result<Vec<Dx7Patch>> {
    let data = fs::read(filename)
        .map_err(|e| anyhow!("Failed to read SYSEX file '{}': {}", filename, e))?;

    parse_sysex_data(&data)
}

/// Parse SYSEX data and extract DX7 patches
pub fn parse_sysex_data(data: &[u8]) -> Result<Vec<Dx7Patch>> {
    if data.is_empty() {
        return Err(anyhow!("Empty SYSEX data"));
    }

    let mut patches = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Find SYSEX start
        while pos < data.len() && data[pos] != SYSEX_START {
            pos += 1;
        }

        if pos >= data.len() {
            break;
        }

        // Find SYSEX end
        let start = pos;
        pos += 1;
        while pos < data.len() && data[pos] != SYSEX_END {
            pos += 1;
        }

        if pos >= data.len() {
            return Err(anyhow!("Unterminated SYSEX message"));
        }

        pos += 1; // Skip SYSEX_END

        let sysex_msg = &data[start..pos];
        if let Ok(patch_data) = parse_sysex_message(sysex_msg) {
            patches.extend(patch_data);
        }
    }

    if patches.is_empty() {
        return Err(anyhow!("No valid DX7 patches found in SYSEX data"));
    }

    Ok(patches)
}

/// Parse a single SYSEX message
fn parse_sysex_message(msg: &[u8]) -> Result<Vec<Dx7Patch>> {
    if msg.len() < 6 {
        return Err(anyhow!("SYSEX message too short"));
    }

    if msg[0] != SYSEX_START {
        return Err(anyhow!("Invalid SYSEX start"));
    }

    if msg[msg.len() - 1] != SYSEX_END {
        return Err(anyhow!("Invalid SYSEX end"));
    }

    if msg[1] != YAMAHA_ID {
        return Err(anyhow!("Not a Yamaha SYSEX message"));
    }

    // Skip channel (msg[2]) and check format
    let format = msg[3];

    match format {
        DX7_SINGLE_VOICE => {
            // Single voice: F0 43 0n 00 [155 bytes] [checksum] F7
            if msg.len() < 163 {
                // 6 header + 155 data + 1 checksum + 1 end
                return Err(anyhow!("Single voice SYSEX too short"));
            }

            let voice_data = &msg[6..161]; // Skip header, take 155 bytes
            let patch = Dx7Patch::from_data(voice_data)?;
            Ok(vec![patch])
        }

        DX7_32_VOICES => {
            // 32 voice bank: F0 43 0n 09 [4096 bytes] [checksum] F7
            if msg.len() < 4104 {
                // 6 header + 4096 data + 1 checksum + 1 end
                return Err(anyhow!("32 voice bank SYSEX too short"));
            }

            let bank_data = &msg[6..4102]; // Skip header, take 4096 bytes
            parse_voice_bank(bank_data)
        }

        _ => Err(anyhow!("Unsupported SYSEX format: 0x{:02X}", format)),
    }
}

/// Parse a 32-voice bank (4096 bytes of packed voice data)
fn parse_voice_bank(bank_data: &[u8]) -> Result<Vec<Dx7Patch>> {
    if bank_data.len() < 4096 {
        return Err(anyhow!("Voice bank data too short"));
    }

    let mut patches = Vec::new();

    for voice_num in 0..32 {
        // Each voice in the bank is 128 bytes (packed format)
        let packed_start = voice_num * 128;
        let packed_voice = &bank_data[packed_start..packed_start + 128];

        // Unpack the voice data from 128 bytes to 155 bytes
        let unpacked = unpack_voice_data(packed_voice)?;
        let patch = Dx7Patch::from_data(&unpacked)?;
        patches.push(patch);
    }

    Ok(patches)
}

/// Unpack voice data from 128-byte bank format to 155-byte single voice format
/// Implementation matches C++ dexed PluginData.cpp:unpackProgram exactly
fn unpack_voice_data(packed: &[u8]) -> Result<Vec<u8>> {
    if packed.len() < 128 {
        return Err(anyhow!("Packed voice data too short"));
    }

    log::debug!("SYSEX: Unpacking voice data, packed[0..20]: {:?}", &packed[..20]);

    let mut unpacked = vec![0u8; 155];

    // Operators (6 operators * 17 bytes packed -> 21 bytes unpacked)
    for op in 0..6 {
        let bulk_base = op * 17;      // Source: packed format
        let unpack_base = op * 21;    // Dest: unpacked format

        // Copy first 11 bytes directly (EG rates/levels, scaling params)
        for i in 0..11 {
            unpacked[unpack_base + i] = packed[bulk_base + i];
        }

        // Unpack bit-packed parameters following C++ dexed logic exactly:

        // Left/right curves from byte 11 (C++: leftrightcurves)
        let leftrightcurves = packed[bulk_base + 11] & 0x0F;
        unpacked[unpack_base + 11] = leftrightcurves & 3;           // Left curve
        unpacked[unpack_base + 12] = (leftrightcurves >> 2) & 3;   // Right curve

        // Detune & Rate Scaling from byte 12 (C++: detune_rs)
        let detune_rs = packed[bulk_base + 12] & 0x7F;
        unpacked[unpack_base + 13] = detune_rs & 7;                 // Rate scaling

        // Key Velocity & Amp Mod Sensitivity from byte 13 (C++: kvs_ams)
        let kvs_ams = packed[bulk_base + 13] & 0x1F;
        unpacked[unpack_base + 14] = kvs_ams & 3;                   // Amp mod sens
        unpacked[unpack_base + 15] = (kvs_ams >> 2) & 7;           // Velocity sens

        // Output level from byte 14 (C++: bulk[op * 17 + 14])
        unpacked[unpack_base + 16] = packed[bulk_base + 14] & 0x7F;

        // Frequency coarse & mode from byte 15 (C++: fcoarse_mode)
        let fcoarse_mode = packed[bulk_base + 15] & 0x3F;
        unpacked[unpack_base + 17] = fcoarse_mode & 1;              // Freq mode
        unpacked[unpack_base + 18] = (fcoarse_mode >> 1) & 0x1F;   // Freq coarse

        // Fine frequency from byte 16 (C++: bulk[op * 17 + 16])
        unpacked[unpack_base + 19] = packed[bulk_base + 16] & 0x7F;

        // Detune from upper bits of byte 12 (C++: (detune_rs >> 3) & 0x7F)
        unpacked[unpack_base + 20] = (detune_rs >> 3) & 0x7F;
    }

    // Global parameters (126-144 in unpacked format)
    unpacked[126..145].copy_from_slice(&packed[102..121]);

    // Voice name (10 bytes)
    unpacked[145..155].copy_from_slice(&packed[118..128]);

    log::debug!("SYSEX: Unpacked data[0..20]: {:?}", &unpacked[..20]);

    Ok(unpacked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data() {
        let result = parse_sysex_data(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_voice_creation() {
        let mut data = [0u8; 155];
        // Set a name
        data[145..155].copy_from_slice(b"TEST VOICE");

        let patch = Dx7Patch::from_data(&data).unwrap();
        assert_eq!(patch.name, "TEST VOICE");
    }

    #[test]
    fn test_operator_params() {
        let mut data = [0u8; 155];
        // Set some operator 0 data
        data[0] = 50; // R1
        data[4] = 99; // L1
        data[16] = 80; // Output level

        let patch = Dx7Patch::from_data(&data).unwrap();
        let op = patch.get_operator(0).unwrap();

        assert_eq!(op.rates.attack, 50);
        assert_eq!(op.levels.attack, 99);
        assert_eq!(op.output_level, 80);
    }

    #[test]
    fn test_global_params() {
        let mut data = [0u8; 155];
        data[134] = 5; // Algorithm
        data[135] = 7; // Feedback
        data[137] = 50; // LFO speed

        let patch = Dx7Patch::from_data(&data).unwrap();
        let global = patch.get_global();

        assert_eq!(global.algorithm, 5);
        assert_eq!(global.feedback, 7);
        assert_eq!(global.lfo_speed, 50);
    }
}
