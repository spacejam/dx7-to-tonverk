use dx7tv::sysex::Dx7Patch;
use dx7tv::synth::Dx7Synth;

/// Debug test to examine envelope levels and gain calculations step by step

#[test]
fn test_envelope_level_to_gain_conversion() {
    let _ = env_logger::try_init();

    println!("=== Envelope Level to Gain Conversion Debug ===");

    // Create a simple test patch to isolate the issue
    let mut patch = Dx7Patch::new("ENV_DEBUG");
    patch.global.algorithm = 31; // Algorithm 32 (simple parallel)

    // Configure just operator 0 with known values
    patch.operators[0].output_level = 50; // Mid-level
    patch.operators[0].coarse_freq = 1;
    patch.operators[0].fine_freq = 0;
    patch.operators[0].detune = 7;

    // Set envelope parameters with predictable values
    patch.operators[0].rates.attack = 99;  // Fast attack
    patch.operators[0].rates.decay1 = 50;
    patch.operators[0].rates.decay2 = 40;
    patch.operators[0].rates.release = 30;

    patch.operators[0].levels.attack = 99;  // Full attack level
    patch.operators[0].levels.decay1 = 75;
    patch.operators[0].levels.decay2 = 50;
    patch.operators[0].levels.release = 0;

    // Disable other operators
    for i in 1..6 {
        patch.operators[i].output_level = 0;
    }

    let mut synth = Dx7Synth::new(44100.0, 0.1);
    synth.load_patch(patch).expect("Failed to load patch");

    // Render just a few samples to see initial values
    let samples = synth.render_note(60, 100, 0.01).expect("Failed to render");

    println!("Generated {} samples", samples.len());
    println!("First 10 sample values:");
    for (i, &sample) in samples.iter().take(10).enumerate() {
        println!("  Sample {}: {:.8}", i, sample);
    }

    let dc_offset = samples.iter().map(|&x| x as f64).sum::<f64>() / samples.len() as f64;
    let rms = (samples.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / samples.len() as f64).sqrt();
    let peak = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("\nSample statistics:");
    println!("  DC offset: {:.8}", dc_offset);
    println!("  RMS: {:.6}", rms);
    println!("  Peak: {:.6}", peak);

    // Check if envelope produces reasonable levels
    let non_zero_samples = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("  Non-zero samples: {}/{}", non_zero_samples, samples.len());

    // Expected behavior:
    // - Should have some non-zero samples
    // - DC offset should be small (< 0.1)
    // - Peak should be reasonable (< 1.0 for normalized audio)

    if dc_offset.abs() > 0.1 {
        println!("⚠️  Significant DC offset detected: {:.6}", dc_offset);
    } else {
        println!("✅ DC offset within acceptable range: {:.6}", dc_offset);
    }

    if peak > 1.0 {
        println!("⚠️  Audio peak exceeds 1.0: {:.3}", peak);
        println!("   This suggests gain calculation issues");
    } else {
        println!("✅ Audio peak within range: {:.3}", peak);
    }

    if non_zero_samples == 0 {
        println!("⚠️  All samples are zero - synthesis not working");
    } else if non_zero_samples < samples.len() / 4 {
        println!("⚠️  Mostly silent - only {:.1}% samples have audio",
                 (non_zero_samples as f64 / samples.len() as f64) * 100.0);
    } else {
        println!("✅ Good audio generation - {:.1}% samples have audio",
                 (non_zero_samples as f64 / samples.len() as f64) * 100.0);
    }

    // Test with different operator output levels
    println!("\n=== Testing Different Output Levels ===");
    for output_level in [0, 25, 50, 75, 99] {
        let mut test_patch = Dx7Patch::new("LEVEL_TEST");
        test_patch.global.algorithm = 31;

        test_patch.operators[0].output_level = output_level;
        test_patch.operators[0].coarse_freq = 1;
        test_patch.operators[0].rates.attack = 99;
        test_patch.operators[0].levels.attack = 99;

        for i in 1..6 {
            test_patch.operators[i].output_level = 0;
        }

        synth.load_patch(test_patch).expect("Failed to load test patch");
        let test_samples = synth.render_note(60, 100, 0.01).expect("Failed to render test");

        let test_peak = test_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        let test_rms = (test_samples.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / test_samples.len() as f64).sqrt();

        println!("Output level {:2}: peak={:.6}, rms={:.6}", output_level, test_peak, test_rms);
    }

    println!("\n✅ Envelope level debug test completed");
}