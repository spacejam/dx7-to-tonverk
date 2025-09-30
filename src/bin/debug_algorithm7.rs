use dx7tv::{render_patch, Dx7Patch};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Test algorithm 7 with operator 0 (the actual feedback operator)
    let mut patch = Dx7Patch::new("ALG7TEST");

    // Algorithm 7 (0-based = 6)
    patch.global.algorithm = 6;
    patch.global.feedback = 7; // Maximum feedback

    // Configure operator 0 (the feedback operator in algorithm 7)
    patch.operators[0].output_level = 99;
    patch.operators[0].rates.attack = 99;   // Very fast attack
    patch.operators[0].rates.decay1 = 50;   // Medium decay
    patch.operators[0].rates.decay2 = 30;   // Medium decay
    patch.operators[0].rates.release = 50;  // Medium release
    patch.operators[0].levels.attack = 99;  // Full level
    patch.operators[0].levels.decay1 = 99;  // Full level
    patch.operators[0].levels.decay2 = 99;  // Full level
    patch.operators[0].levels.release = 0;  // Silent on release
    patch.operators[0].coarse_freq = 1;     // 1:1 frequency ratio
    patch.operators[0].fine_freq = 0;       // No fine tuning
    patch.operators[0].detune = 7;          // Center detune

    // Render MIDI note 69 (A4, ~440 Hz)
    let samples = render_patch(patch, 69, 1.0)?;

    // Calculate RMS level
    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    println!("Generated {} samples, RMS level: {}", samples.len(), rms);

    // Check for non-zero samples
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-8).count();
    println!("Non-zero samples: {} / {}", non_zero_count, samples.len());

    if non_zero_count == 0 {
        println!("ERROR: All samples are zero!");
    } else {
        println!("SUCCESS: Algorithm 7 with operator 0 feedback produced audio");
        // Show first few non-zero samples
        for (i, &sample) in samples.iter().enumerate().take(20) {
            if sample.abs() > 1e-8 {
                println!("Sample {}: {}", i, sample);
            }
        }
    }

    Ok(())
}