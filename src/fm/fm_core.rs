
//! FM synthesis core - the main synthesis engine
//!
//! This module coordinates all the components to produce the final
//! FM synthesis output, managing multiple voices and global parameters.

use super::{dx7note::Dx7Note, lfo::Lfo, controllers::Controllers, constants::N};
use log::{debug, trace};

/// Voice management for polyphonic synthesis
#[derive(Clone, Debug)]
pub struct Voice {
    pub note: Dx7Note,
    pub age: u32,        // For voice stealing
    pub midi_note: u8,
    pub midi_channel: u8,
}

impl Voice {
    pub fn new() -> Self {
        Self {
            note: Dx7Note::new(),
            age: 0,
            midi_note: 0,
            midi_channel: 0,
        }
    }

    pub fn is_free(&self) -> bool {
        !self.note.is_active()
    }

    pub fn trigger(&mut self, midi_note: u8, velocity: u8, channel: u8, age: u32, patch_data: &[u8]) {
        self.midi_note = midi_note;
        self.midi_channel = channel;
        self.age = age;
        self.note.init(midi_note, velocity);
        if patch_data.len() >= 155 {
            self.note.apply_patch(patch_data);
        }
    }

    pub fn release(&mut self) {
        self.note.release();
    }
}

/// Main FM synthesis core
#[derive(Clone, Debug)]
pub struct FmCore {
    /// Polyphonic voices
    voices: Vec<Voice>,

    /// Global LFO
    lfo: Lfo,

    /// Global controllers
    controllers: Controllers,

    /// Voice allocation counter
    voice_counter: u32,

    /// Maximum polyphony
    max_voices: usize,

    /// Current patch data
    patch_data: [u8; 155], // DX7 patch is 155 bytes
}

impl Default for FmCore {
    fn default() -> Self {
        Self::new(16) // 16-voice polyphony by default
    }
}

impl FmCore {
    /// Create a new FM core with specified polyphony
    pub fn new(max_voices: usize) -> Self {
        let voices = (0..max_voices).map(|_| Voice::new()).collect();

        Self {
            voices,
            lfo: Lfo::new(),
            controllers: Controllers::new(),
            voice_counter: 0,
            max_voices,
            patch_data: [0; 155],
        }
    }

    /// Process audio for N samples
    pub fn process(&mut self, output: &mut [i32]) {
        assert_eq!(output.len(), N);

        // Clear output buffer
        output.fill(0);

        // Process each active voice
        let mut active_voices = 0;
        for (_i, voice) in self.voices.iter_mut().enumerate() {
            if voice.note.is_active() {
                voice.note.process(output, &self.lfo);
                voice.age += 1;
                active_voices += 1;
            }
        }

        static mut FIRST_CALL: bool = true;
        unsafe {
            if FIRST_CALL {
                log::debug!("FM_CORE: First process call - found {} active voices out of {}", active_voices, self.voices.len());
                FIRST_CALL = false;
            }
        }

        // Debug logging - check intermediate values
        if active_voices > 0 {
            let sample_before_volume = output[0];

            // Apply global volume and limiting
            let volume = self.controllers.get_volume_amount();
            for sample in output.iter_mut() {
                *sample = (*sample as f32 * volume) as i32;
                *sample = (*sample).clamp(-(1 << 23), (1 << 23) - 1); // Clamp to 24-bit range
            }

            static mut DEBUG_COUNTER: i32 = 0;
            unsafe {
                DEBUG_COUNTER += 1;
                if DEBUG_COUNTER <= 5 {
                    log::debug!("FM_CORE DEBUG {}: active_voices={}, sample_before_volume={}, volume={}, sample_after_volume={}",
                               DEBUG_COUNTER, active_voices, sample_before_volume, volume, output[0]);
                }
            }
        }
    }

    /// Trigger a note
    pub fn note_on(&mut self, midi_note: u8, velocity: u8, channel: u8) {
        log::debug!("FM_CORE: note_on called, patch_data[0..20]: {:?}", &self.patch_data[..20]);

        // Find a free voice or steal the oldest
        let voice_index = self.find_voice_for_note(midi_note, channel);

        if let Some(voice) = self.voices.get_mut(voice_index) {
            self.voice_counter += 1;
            log::debug!("FM_CORE: Calling trigger on voice {}, patch_data len: {}", voice_index, self.patch_data.len());
            voice.trigger(midi_note, velocity, channel, self.voice_counter, &self.patch_data);
            log::debug!("FM_CORE: Voice {} active after trigger: {}", voice_index, voice.note.is_active());
        } else {
            log::debug!("FM_CORE: No voice available for note {}", midi_note);
        }
    }

    /// Release a note
    pub fn note_off(&mut self, midi_note: u8, channel: u8) {
        for voice in &mut self.voices {
            if voice.midi_note == midi_note &&
               voice.midi_channel == channel &&
               voice.note.is_active() {
                voice.release();
            }
        }
    }

    /// Find the best voice to use for a new note
    fn find_voice_for_note(&mut self, _midi_note: u8, _channel: u8) -> usize {
        // First, try to find a free voice
        for (i, voice) in self.voices.iter().enumerate() {
            if voice.is_free() {
                return i;
            }
        }

        // If no free voice, steal the oldest
        self.voices.iter()
            .enumerate()
            .min_by_key(|(_, voice)| voice.age)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Set pitch bend
    pub fn set_pitch_bend(&mut self, value: u16) {
        self.controllers.set_pitch_bend(value);
        let bend_semitones = self.controllers.get_pitch_bend_semitones(2.0); // ±2 semitones

        // Apply to all active voices
        for voice in &mut self.voices {
            if voice.note.is_active() {
                voice.note.set_pitch_bend(bend_semitones * 100.0); // Convert to cents
            }
        }
    }

    /// Set modulation wheel
    pub fn set_mod_wheel(&mut self, value: u8) {
        self.controllers.set_mod_wheel(value);
        // TODO: Apply modulation to active voices
    }

    /// Set volume
    pub fn set_volume(&mut self, value: u8) {
        self.controllers.set_volume(value);
    }

    /// Load a DX7 patch
    pub fn load_patch(&mut self, patch_data: &[u8]) {
        debug!("FM_CORE: load_patch called with {} bytes", patch_data.len());
        trace!("FM_CORE: First 20 bytes: {:?}", &patch_data[..20.min(patch_data.len())]);
        if patch_data.len() >= 155 {
            self.patch_data[..155].copy_from_slice(&patch_data[..155]);
            trace!("FM_CORE: Copied patch data, self.patch_data[0..20]: {:?}", &self.patch_data[..20]);
            self.apply_patch_parameters();
        } else {
            debug!("FM_CORE: Patch data too short: {} < 155", patch_data.len());
        }
    }

    /// Apply currently loaded patch parameters to all voices
    fn apply_patch_parameters(&mut self) {
        // Apply patch to all voices
        for voice in &mut self.voices {
            voice.note.apply_patch(&self.patch_data);
        }
    }

    /// All notes off (panic)
    pub fn all_notes_off(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
    }

    /// Reset all controllers
    pub fn reset_controllers(&mut self) {
        self.controllers.reset();
    }

    /// Get number of active voices
    pub fn get_active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.note.is_active()).count()
    }

    /// Set LFO parameters
    pub fn set_lfo_params(&mut self, params: &[u8; 6]) {
        self.lfo.reset(params);
    }

    /// Initialize sample rate dependent parameters
    pub fn init_sample_rate(&mut self, sample_rate: f64) {
        Lfo::init(sample_rate);
        super::env::Env::init_sr(sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fm_core_creation() {
        let core = FmCore::new(8);
        assert_eq!(core.max_voices, 8);
        assert_eq!(core.voices.len(), 8);
        assert_eq!(core.get_active_voice_count(), 0);
    }

    #[test]
    fn test_note_on_off() {
        let mut core = FmCore::new(4);

        // Trigger a note
        core.note_on(60, 100, 0); // C4, forte, channel 0
        assert_eq!(core.get_active_voice_count(), 1);

        // Release the note
        core.note_off(60, 0);
        // Note might still be active in release phase
    }

    #[test]
    fn test_polyphony() {
        let mut core = FmCore::new(2); // 2-voice polyphony

        // Trigger two notes
        core.note_on(60, 100, 0);
        core.note_on(64, 100, 0);
        assert!(core.get_active_voice_count() <= 2);

        // Trigger third note (should steal a voice)
        core.note_on(67, 100, 0);
        assert!(core.get_active_voice_count() <= 2);
    }

    #[test]
    fn test_controllers() {
        let mut core = FmCore::new(4);

        core.set_pitch_bend(0x3000); // Some pitch bend
        core.set_mod_wheel(64);
        core.set_volume(100);

        assert_eq!(core.controllers.pitch_bend, 0x3000);
        assert_eq!(core.controllers.mod_wheel, 64);
        assert_eq!(core.controllers.volume, 100);
    }

    #[test]
    fn test_all_notes_off() {
        let mut core = FmCore::new(4);

        // Trigger some notes
        core.note_on(60, 100, 0);
        core.note_on(64, 100, 0);

        // Panic
        core.all_notes_off();

        // All voices should be released
        // (They might still be active in release phase)
    }

    #[test]
    fn test_process() {
        let mut core = FmCore::new(2);
        let mut output = [0i32; N];

        // Process silence
        core.process(&mut output);
        // Should not crash

        // Trigger a note and process
        core.note_on(69, 100, 0); // A4
        core.process(&mut output);
        // Should not crash
    }
}