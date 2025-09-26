//! DX7TV - DX7 Test Vector CLI
//!
//! Generate WAV files from DX7 SYSEX patches using FM synthesis.
//! This is a consolidated version that includes all necessary FM synthesis
//! components without DAW plugin dependencies.

/// Initialize logging for the library
pub fn init_logging() {
    env_logger::init();
}

// FM synthesis engine modules
pub mod fm {
    //! FM synthesis engine - core DX7 synthesis implementation
    pub mod constants;
    pub mod sin;
    pub mod exp2;
    pub mod env;
    pub mod lfo;
    pub mod pitchenv;
    pub mod porta;
    pub mod controllers;
    pub mod fm_op_kernel;
    pub mod dx7note;
    pub mod fm_core;
    pub mod freqlut;
    pub mod tuning;

    // Re-export commonly used items
    pub use constants::*;
    pub use dx7note::Dx7Note;
    pub use fm_core::FmCore;
    pub use env::Env;
    pub use lfo::Lfo;
    pub use controllers::Controllers;
    pub use freqlut::FreqLut;
}

// Application modules
pub mod sysex;
pub mod synth;
pub mod wav_writer;

// Re-export main types
pub use synth::Dx7Synth;
pub use sysex::{Dx7Patch, parse_sysex_data, parse_sysex_file};
pub use wav_writer::WavOutput;