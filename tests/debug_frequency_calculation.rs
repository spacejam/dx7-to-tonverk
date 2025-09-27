use anyhow::Result;
use dx7tv::Dx7Synth;

/// Debug test to see what frequency is being calculated
#[test]
fn debug_frequency_calculation() -> Result<()> {
    // Initialize log to see debug output
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let sample_rate = 44100.0;
    let note_length = 0.1;

    // Create a simple patch with only one carrier operator producing a sine wave
    let mut synth = Dx7Synth::new(sample_rate, note_length + 0.1);

    // Create a minimal test patch - single operator producing a sine wave
    let mut patch = dx7tv::sysex::Dx7Patch::new("DEBUG");

    // Set algorithm 1 (stored as 0) - single carrier
    patch.global.algorithm = 0;

    // Configure operator 5 (maps to sysex operator 0 - carrier in algorithm 1) for debugging
    patch.operators[5].rates.attack = 99;    // Maximum attack rate for instant sound
    patch.operators[5].rates.decay1 = 99;    // Fast decay1
    patch.operators[5].rates.decay2 = 99;    // Fast decay2
    patch.operators[5].rates.release = 50;   // Medium release

    patch.operators[5].levels.attack = 99;   // Maximum attack level
    patch.operators[5].levels.decay1 = 99;   // Full level
    patch.operators[5].levels.decay2 = 99;   // Full level
    patch.operators[5].levels.release = 0;   // Silent release

    patch.operators[5].output_level = 99;    // Maximum output
    patch.operators[5].coarse_freq = 1;      // 1:1 frequency ratio
    patch.operators[5].fine_freq = 0;        // No fine tuning
    patch.operators[5].detune = 7;           // Center detune
    patch.operators[5].osc_mode = 0;         // Ratio mode (follows MIDI note)

    // Ensure all other operators are silent
    for i in 0..5 {
        patch.operators[i].output_level = 0;
    }

    synth.load_patch(patch)?;

    println!("Testing MIDI note 60 (middle C)...");
    let _samples_60 = synth.render_note(60, 127, note_length)?;

    println!("Testing MIDI note 72 (C5)...");
    synth.reset();
    let _samples_72 = synth.render_note(72, 127, note_length)?;

    Ok(())
}