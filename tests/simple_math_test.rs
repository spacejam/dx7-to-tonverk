//! Simple math test to establish baseline

use std::f64::consts::PI;

#[test]
fn test_expected_middle_c_calculation() {
    println!("=== Expected Middle C Calculation ===");

    // This should match what MIDI note 60 produces
    let midi_note = 60;
    let expected_freq = 440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0);

    println!("MIDI note {} -> {:.2} Hz", midi_note, expected_freq);
    assert!((expected_freq - 261.63).abs() < 0.1, "MIDI note 60 should be ~261.63 Hz");

    // Test what DX7 coarse=0 should produce (0.5x ratio)
    let coarse_0_freq = expected_freq * 0.5;
    println!("With coarse=0 (0.5x): {:.2} Hz", coarse_0_freq);
    assert!((coarse_0_freq - 130.81).abs() < 1.0, "Coarse 0 should be ~130.81 Hz");

    println!("Math test: PASS");
}

#[test]
fn test_sine_wave_samples() {
    println!("=== Expected Sine Wave Samples ===");

    let frequency = 261.63; // Middle C
    let sample_rate = 44100.0;
    let amplitude = 0.5;

    println!("Generating {:.2} Hz sine wave at {} sample rate", frequency, sample_rate);
    println!("First 10 samples should show clear sine pattern:");

    for i in 0..10 {
        let t = i as f64 / sample_rate;
        let sample = amplitude * (2.0 * PI * frequency * t).sin();
        println!("Sample {}: t={:.6}s, value={:.6}", i, t, sample);
    }

    // Period check
    let period_samples = sample_rate / frequency;
    println!("Complete cycle every {:.1} samples", period_samples);

    println!("Expected sine pattern: PASS");
}

#[test]
fn test_phase_increment_calculation() {
    println!("=== Phase Increment Test ===");

    // This tests the phase increment calculation used in synthesis
    let freq_hz = 130.30; // What we get from coarse=0
    let sample_rate = 44100.0;

    // Current Rust calculation
    let phase_inc = ((freq_hz * 65536.0) / sample_rate) as i32;
    println!("Frequency: {:.2} Hz", freq_hz);
    println!("Phase increment: {}", phase_inc);

    // Should be around 193-194 for this frequency
    assert!(phase_inc >= 190 && phase_inc <= 200, "Phase increment should be ~193");

    // What this means for synthesis
    println!("Phase advances by {} per sample", phase_inc);
    println!("Will complete cycle every ~{} samples", 65536 / phase_inc);

    let expected_cycle_samples = sample_rate / freq_hz;
    let actual_cycle_samples = 65536.0 / phase_inc as f64;

    println!("Expected cycle: {:.1} samples", expected_cycle_samples);
    println!("Actual cycle: {:.1} samples", actual_cycle_samples);

    println!("Phase increment: PASS");
}