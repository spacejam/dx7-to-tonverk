//! Simple step-by-step DX7 synthesis test
//! This creates a minimal version that matches the C++ audio path exactly

use std::f64::consts::PI;

/// Simple test: Generate one pure sine wave at middle C
pub fn test_simple_sine() {
    println!("=== Simple Sine Test (Middle C = 261.63 Hz) ===");

    let sample_rate = 44100.0;
    let frequency = 261.63; // Middle C
    let amplitude = 0.5;
    let duration = 0.1; // 100ms

    println!("Frequency: {:.2} Hz", frequency);
    println!("Sample rate: {} Hz", sample_rate);
    println!("Duration: {}s", duration);

    let samples_to_generate = (sample_rate * duration) as usize;
    println!("Total samples: {}", samples_to_generate);
    println!();

    println!("First 10 samples:");
    for i in 0..10 {
        let t = i as f64 / sample_rate;
        let sample = amplitude * (2.0 * PI * frequency * t).sin();
        println!("Sample {}: t={:.6}s, value={:.6}", i, t, sample);

        // This should show a clear sine wave pattern
        // Sample 0 should be ~0
        // Sample values should oscillate between -0.5 and +0.5
    }

    // Calculate expected period
    let period_samples = sample_rate / frequency;
    println!();
    println!("Expected period: {:.1} samples", period_samples);
    println!("Should complete one cycle every ~{} samples", period_samples as i32);
}

/// Test DX7 frequency calculation step-by-step
pub fn test_dx7_frequency_calc() {
    println!("=== DX7 Frequency Calculation Test ===");

    // Test what the current Rust implementation calculates
    let midi_note = 60;
    let coarse = 1;  // Should give 1:1 ratio
    let fine = 0;
    let detune = 7;  // Center detune

    // Step 1: Base MIDI frequency
    let base_freq = 440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0);
    println!("MIDI note {}: base frequency = {:.2} Hz", midi_note, base_freq);

    // Step 2: DX7 coarse multipliers (from C++ code)
    let coarse_values = [
        ("0 (0.5x)", 0.5),
        ("1 (1.0x)", 1.0),
        ("2 (2.0x)", 2.0),
        ("3 (3.0x)", 3.0),
    ];

    for (desc, ratio) in coarse_values.iter() {
        let final_freq = base_freq * ratio;
        println!("Coarse {}: {:.2} Hz", desc, final_freq);
    }

    println!();
    println!("Expected for coarse=1: {:.2} Hz (should be middle C)", base_freq);
}

/// Test what our current implementation actually produces
pub fn test_current_implementation() {
    println!("=== Current Implementation Test ===");
    println!("This will show what frequencies our Rust code actually generates");
    println!();

    // We need to call into our actual implementation here
    // For now, just show what we expect to see
    println!("TODO: Call actual Rust dx7tv code and show:");
    println!("1. What frequency it calculates for MIDI note 60");
    println!("2. What the first 10 audio samples look like");
    println!("3. Whether the sine wave period matches the expected frequency");
}

pub fn run_all_tests() {
    println!("DX7 Audio Path Step-by-Step Test");
    println!("=================================");
    println!();

    test_simple_sine();
    println!();

    test_dx7_frequency_calc();
    println!();

    test_current_implementation();
    println!();

    println!("=== Analysis ===");
    println!("1. The simple sine test shows what a clear 261.63 Hz tone should look like");
    println!("2. The DX7 frequency test shows what different coarse values should produce");
    println!("3. Compare this with the actual output from dx7tv to find the discrepancy");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all() {
        run_all_tests();
    }
}