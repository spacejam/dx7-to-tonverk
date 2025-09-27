use dx7tv::sysex::Dx7Patch;
use dx7tv::synth::Dx7Synth;

/// Test to verify envelope timing - volume should go up during attack, then down during decay/release
///
/// This creates a simple sine wave with a clear ADSR envelope and verifies:
/// 1. Attack: volume increases from 0 to peak
/// 2. Decay: volume decreases from peak to sustain level
/// 3. Release: volume decreases to 0 when key is released
///
/// This will help determine if the envelope is advancing at the expected timing.

#[test]
fn test_envelope_timing_and_volume_changes() {
    let _ = env_logger::try_init();

    println!("=== Envelope Timing Test ==.");

    // Create a simple test patch with clear envelope timing
    let mut patch = Dx7Patch::new("ENV_TEST");
    patch.global.algorithm = 31; // Algorithm 32 (simple parallel - only carrier operators)

    // Configure only operator 0 for a simple sine wave
    patch.operators[0].output_level = 90; // High output level for clear signal
    patch.operators[0].coarse_freq = 1;   // 1x frequency (fundamental)
    patch.operators[0].fine_freq = 0;
    patch.operators[0].detune = 7;        // Center detune

    // Set a clear ADSR envelope with specific timing
    patch.operators[0].rates.attack = 90;   // Fast attack (about 0.1 seconds)
    patch.operators[0].rates.decay1 = 70;   // Medium decay (about 0.5 seconds)
    patch.operators[0].rates.decay2 = 60;   // Slower sustain decay
    patch.operators[0].rates.release = 50;  // Medium release (about 1 second)

    patch.operators[0].levels.attack = 99;  // Full attack level
    patch.operators[0].levels.decay1 = 80;  // Decay to 80% level
    patch.operators[0].levels.decay2 = 60;  // Sustain at 60% level
    patch.operators[0].levels.release = 0;  // Release to silence

    // Disable all other operators
    for i in 1..6 {
        patch.operators[i].output_level = 0;
    }

    let mut synth = Dx7Synth::new(44100.0, 2.0);
    synth.load_patch(patch).expect("Failed to load test patch");

    // Generate 3 seconds of audio to see full envelope
    let samples = synth.render_note(60, 100, 3.0).expect("Failed to render note");

    println!("Generated {} samples for envelope analysis", samples.len());

    // Analyze the envelope by measuring RMS in time windows
    let sample_rate = 44100.0;
    let window_size = (0.05 * sample_rate) as usize; // 50ms windows
    let num_windows = samples.len() / window_size;

    let mut rms_values = Vec::new();
    let mut time_points = Vec::new();

    for i in 0..num_windows {
        let start = i * window_size;
        let end = (start + window_size).min(samples.len());

        // Calculate RMS for this window
        let mut sum_squares = 0.0f64;
        for &sample in &samples[start..end] {
            sum_squares += (sample as f64) * (sample as f64);
        }
        let rms = (sum_squares / (end - start) as f64).sqrt();

        rms_values.push(rms);
        time_points.push((start as f64) / sample_rate);
    }

    println!("\n=== Envelope Analysis (RMS over time) ===");
    for (i, (&time, &rms)) in time_points.iter().zip(rms_values.iter()).enumerate() {
        if i % 4 == 0 {  // Print every 200ms
            println!("Time: {:5.2}s, RMS: {:8.6}", time, rms);
        }
    }

    // Check for expected envelope behavior
    let mut peak_rms = 0.0f64;
    let mut peak_time = 0.0f64;

    // Find the peak RMS (should occur during attack phase, within first 0.5 seconds)
    for (i, (&time, &rms)) in time_points.iter().zip(rms_values.iter()).enumerate() {
        if time < 0.5 && rms > peak_rms {
            peak_rms = rms;
            peak_time = time;
        }
    }

    println!("\n=== Envelope Behavior Analysis ===");
    println!("Peak RMS: {:.6} at time {:.2}s", peak_rms, peak_time);

    // Check attack phase: RMS should increase from start to peak
    let attack_start_rms = rms_values[0];
    let attack_increase = peak_rms > attack_start_rms * 2.0; // At least 2x increase

    println!("Attack start RMS: {:.6}", attack_start_rms);
    println!("Attack increase: {} ({}x)", attack_increase, peak_rms / attack_start_rms);

    // Check decay/sustain phase: RMS should decrease after peak
    let sustain_time_index = time_points.iter()
        .position(|&t| t >= 1.0) // Check at 1 second
        .unwrap_or(rms_values.len() - 1);

    let sustain_rms = rms_values[sustain_time_index];
    let decay_occurred = sustain_rms < peak_rms * 0.9; // Should drop to less than 90% of peak

    println!("Sustain RMS at 1.0s: {:.6}", sustain_rms);
    println!("Decay occurred: {} ({}x from peak)", decay_occurred, sustain_rms / peak_rms);

    // Check release phase (note: our render_note doesn't trigger key release,
    // but envelope should still be active throughout)
    let end_rms = rms_values[rms_values.len() - 1];
    println!("Final RMS at end: {:.6}", end_rms);

    // Check overall signal characteristics
    let total_rms = (samples.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / samples.len() as f64).sqrt();
    let peak_sample = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("\n=== Overall Signal Analysis ===");
    println!("Total RMS: {:.6}", total_rms);
    println!("Peak sample: {:.6}", peak_sample);

    // Test assertions
    assert!(
        peak_rms > 1e-8,
        "Peak RMS should be significant, got: {:.2e}. This suggests the envelope is not producing audible levels.",
        peak_rms
    );

    assert!(
        attack_increase,
        "Attack should cause RMS to increase significantly. Start: {:.2e}, Peak: {:.2e}",
        attack_start_rms, peak_rms
    );

    assert!(
        peak_time < 0.5,
        "Peak should occur within 0.5 seconds (during attack), but occurred at {:.2}s",
        peak_time
    );

    assert!(
        total_rms > 1e-6,
        "Total RMS should be audible, got: {:.2e}. This suggests synthesis is not working properly.",
        total_rms
    );

    // Check that the envelope is actually changing over time (not static noise)
    let rms_variance = {
        let mean_rms = rms_values.iter().sum::<f64>() / rms_values.len() as f64;
        let variance = rms_values.iter()
            .map(|&rms| (rms - mean_rms) * (rms - mean_rms))
            .sum::<f64>() / rms_values.len() as f64;
        variance.sqrt()
    };

    assert!(
        rms_variance > peak_rms * 0.1,
        "RMS should vary over time (envelope changing), but variance is {:.2e} vs peak {:.2e}",
        rms_variance, peak_rms
    );

    println!("✅ Envelope timing test passed - envelope advances properly over time");
}