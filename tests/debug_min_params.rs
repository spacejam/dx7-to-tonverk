use dx7tv::sysex::Dx7Patch;
use dx7tv::Dx7Synth;

#[test]
fn debug_min_params() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    // Start with a working configuration and gradually reduce parameters
    let mut patch = Dx7Patch::new("DEBUG_MIN");

    // Algorithm 1 (stored as 0) - single carrier (operator 5)
    patch.global.algorithm = 0;

    // Configure all operators for minimal values
    for op in 0..6 {
        if op == 5 {
            // Operator 5 is the carrier in algorithm 1
            patch.operators[op].output_level = 10;   // Small but audible
            patch.operators[op].coarse_freq = 1;     // 1:1 ratio (not 0!)
            patch.operators[op].fine_freq = 0;       // No fine tuning
            patch.operators[op].detune = 7;          // Center detune
            patch.operators[op].osc_mode = 0;        // Ratio mode

            // Set up envelope for immediate sound
            patch.operators[op].rates.attack = 99;   // Fast attack
            patch.operators[op].rates.decay1 = 50;   // Moderate decay
            patch.operators[op].rates.decay2 = 50;   // Moderate decay
            patch.operators[op].rates.release = 50;  // Moderate release

            patch.operators[op].levels.attack = 99;  // Full attack level
            patch.operators[op].levels.decay1 = 99;  // Full sustain
            patch.operators[op].levels.decay2 = 99;  // Full sustain
            patch.operators[op].levels.release = 0;  // Silent release
        } else {
            // All other operators silent
            patch.operators[op].output_level = 0;
        }
    }

    let mut synth = Dx7Synth::new(44100.0, 1.0);
    synth.load_patch(patch).expect("Failed to load patch");

    println!("Testing minimal configuration with coarse_freq=1...");
    let samples = synth.render_note(60, 127, 0.05).expect("Should render");

    let non_zero = samples.iter().filter(|&&x| x.abs() > 1e-8).count();
    let max_amp = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Generated {} samples, {} non-zero, max_amp: {:.6}",
             samples.len(), non_zero, max_amp);

    // Now test with coarse_freq = 0 to see what happens
    synth.reset();
    let mut patch2 = Dx7Patch::new("DEBUG_MIN_ZERO");
    patch2.global.algorithm = 0;

    for op in 0..6 {
        if op == 5 {
            patch2.operators[op].output_level = 10;
            patch2.operators[op].coarse_freq = 0;  // This might be the problem
            patch2.operators[op].fine_freq = 0;
            patch2.operators[op].detune = 7;
            patch2.operators[op].osc_mode = 0;

            patch2.operators[op].rates.attack = 99;
            patch2.operators[op].rates.decay1 = 50;
            patch2.operators[op].rates.decay2 = 50;
            patch2.operators[op].rates.release = 50;

            patch2.operators[op].levels.attack = 99;
            patch2.operators[op].levels.decay1 = 99;
            patch2.operators[op].levels.decay2 = 99;
            patch2.operators[op].levels.release = 0;
        } else {
            patch2.operators[op].output_level = 0;
        }
    }

    synth.load_patch(patch2).expect("Failed to load patch");

    println!("Testing minimal configuration with coarse_freq=0...");
    let samples2 = synth.render_note(60, 127, 0.05);

    match samples2 {
        Ok(audio) => {
            let non_zero2 = audio.iter().filter(|&&x| x.abs() > 1e-8).count();
            let max_amp2 = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
            println!("Generated {} samples, {} non-zero, max_amp: {:.6}",
                     audio.len(), non_zero2, max_amp2);
        }
        Err(e) => {
            println!("Failed to render with coarse_freq=0: {:?}", e);
        }
    }
}