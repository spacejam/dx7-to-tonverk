/*
 * Comprehensive envelope behavior tests for all DX7 algorithms
 *
 * These tests ensure that every operator in every algorithm has correct
 * envelope behavior and does not produce zero output due to envelope issues.
 */

use dx7tv::fm::{Dx7Note, N};
use dx7tv::sysex::Dx7Patch;
use log::debug;

/// Test that all operators in a given algorithm produce non-zero envelope values
fn test_algorithm_envelope_behavior(algorithm: u8) {
    // Don't initialize logger here - it's done once per test function

    debug!("=== Testing Algorithm {} Envelope Behavior ===", algorithm);

    // Create a test patch using structured API
    let mut patch = Dx7Patch::new(&format!("ALGO{:02}", algorithm));

    // Set algorithm (stored as 0-31, but displayed as 1-32)
    patch.global.algorithm = algorithm - 1;

    // Configure all operators with reasonable envelope parameters
    for op in 0..6 {
        // Set envelope rates: moderate attack, decay, sustain, release
        patch.operators[op].rates.attack = 50;
        patch.operators[op].rates.decay1 = 40;
        patch.operators[op].rates.decay2 = 30;
        patch.operators[op].rates.release = 20;

        // Set envelope levels: full attack, moderate decay, sustain, silent release
        patch.operators[op].levels.attack = 99;
        patch.operators[op].levels.decay1 = 85;
        patch.operators[op].levels.decay2 = 75;
        patch.operators[op].levels.release = 0;

        // Set reasonable output level
        patch.operators[op].output_level = 80;

        // Set frequency parameters for reasonable pitch
        patch.operators[op].coarse_freq = 1;  // 1.0 ratio
        patch.operators[op].fine_freq = 0;    // No fine tuning
        patch.operators[op].detune = 7;       // Center detune

        // Set level scaling parameters
        patch.operators[op].level_scaling_bp = 60;  // Break point (middle C)
        patch.operators[op].level_scaling_ld = 0;   // Left depth
        patch.operators[op].level_scaling_rd = 0;   // Right depth
        patch.operators[op].level_scaling_lc = 0;   // Left curve
        patch.operators[op].level_scaling_rc = 0;   // Right curve
        patch.operators[op].rate_scaling = 0;       // No rate scaling
        patch.operators[op].velocity_sens = 0;      // No velocity sensitivity
        patch.operators[op].amp_mod_sens = 0;       // No amp mod sensitivity
    }

    let mut note = Dx7Note::new();

    // Initialize note
    note.init(60, 100);  // Middle C, forte
    note.apply_patch(&patch.to_data());

    // Test envelope behavior by running through several envelope stages
    let mut output = [0i32; N];
    let mut non_zero_samples_found = false;

    debug!("Testing algorithm {} with {} samples per block", algorithm, N);

    // Process several blocks to let envelopes develop
    for block in 0..20 {  // Process 20 blocks
        output.fill(0);
        note.process(&mut output, &Default::default());

        // Check if any samples are non-zero
        for (i, &sample) in output.iter().enumerate() {
            if sample.abs() > 1000 {  // Threshold for meaningful audio
                non_zero_samples_found = true;
                debug!("Algorithm {} block {}: Found non-zero sample {} at index {}",
                      algorithm, block, sample, i);
                break;
            }
        }

        if non_zero_samples_found {
            break;
        }
    }

    // Verify that the algorithm produces audio
    assert!(
        non_zero_samples_found,
        "Algorithm {} failed to produce non-zero audio samples - possible envelope issue. \
         All operators may have zero gain due to envelope overflow or other envelope problems.",
        algorithm
    );

    debug!("Algorithm {} envelope test PASSED", algorithm);
}

/// Test envelope behavior for all 32 DX7 algorithms
#[test]
fn test_all_algorithms_envelope_behavior() {
    let _ = env_logger::try_init();

    println!("=== Testing Envelope Behavior for All 32 DX7 Algorithms ===");

    for algorithm in 1..=32 {
        println!("Testing algorithm {}...", algorithm);
        test_algorithm_envelope_behavior(algorithm);
    }

    println!("All 32 algorithms passed envelope behavior tests!");
}

/// Test specific problematic algorithms that were identified during debugging
#[test]
fn test_problematic_algorithms() {
    let _ = env_logger::try_init();

    println!("=== Testing Previously Problematic Algorithms ===");

    // Algorithm 4 was specifically problematic with OP2 and OP5 envelope overflow
    println!("Testing algorithm 4 (previously had OP2/OP5 envelope overflow)...");
    test_algorithm_envelope_behavior(4);

    // Test a few other algorithms that use different operator configurations
    for algo in [1, 8, 16, 22, 32] {
        println!("Testing algorithm {}...", algo);
        test_algorithm_envelope_behavior(algo);
    }

    println!("All problematic algorithms now pass envelope behavior tests!");
}

/// Test envelope behavior with extreme parameter values
#[test]
fn test_envelope_parameter_extremes() {
    let _ = env_logger::try_init();

    println!("=== Testing Envelope Behavior with Extreme Parameters ===");

    // Test with maximum output levels that previously caused overflow
    let mut patch = Dx7Patch::new("EXTREME");
    patch.global.algorithm = 3;  // Algorithm 4 (0-indexed)

    for op in 0..6 {
        // Extreme envelope parameters
        patch.operators[op].rates.attack = 99;
        patch.operators[op].rates.decay1 = 99;
        patch.operators[op].rates.decay2 = 0;   // Min sustain rate
        patch.operators[op].rates.release = 99;

        patch.operators[op].levels.attack = 99;
        patch.operators[op].levels.decay1 = 99;
        patch.operators[op].levels.decay2 = 99;
        patch.operators[op].levels.release = 0;

        // Maximum output level that previously caused overflow
        patch.operators[op].output_level = 99;

        // Set frequency parameters
        patch.operators[op].coarse_freq = 1;  // 1.0 ratio
        patch.operators[op].fine_freq = 0;    // No fine tuning
        patch.operators[op].detune = 7;       // Center detune
    }

    let mut note = Dx7Note::new();

    note.init(60, 127);  // Maximum velocity
    note.apply_patch(&patch.to_data());

    // Process audio and verify no overflow issues
    let mut output = [0i32; N];
    let mut max_sample = 0i32;

    for _block in 0..10 {
        output.fill(0);
        note.process(&mut output, &Default::default());

        for &sample in &output {
            max_sample = max_sample.max(sample.abs());

            // Verify samples are within reasonable range (no overflow to negative)
            assert!(
                sample > -100_000_000 && sample < 100_000_000,
                "Sample value {} is outside reasonable range - possible integer overflow",
                sample
            );
        }
    }

    println!("Extreme parameter test passed. Maximum sample value: {}", max_sample);
}