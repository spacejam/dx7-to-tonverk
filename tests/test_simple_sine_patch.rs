use dx7tv::{Dx7Synth, sysex::parse_sysex_file};
use anyhow::Result;

#[test]
fn test_simple_sine_patch() -> Result<()> {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    println!("=== SIMPLE SINE PATCH TEST ===");

    // Load the test.syx file
    let patches = parse_sysex_file("test.syx")?;
    let patch_0 = &patches[0];

    println!("Testing patch: '{}', Algorithm: {}", patch_0.name, patch_0.global.algorithm);

    // DEBUG: Print the raw sysex data around algorithm and Op0
    let patch_data = patch_0.to_data();
    println!("Raw sysex data:");
    println!("  Algorithm byte [134]: {} (should be 2 for Algorithm 3)", patch_data[134]);
    println!("  Op0 output level byte [16]: {} (should be high)", patch_data[16]);
    println!("  Op1 output level byte [37]: {} (should be 0)", patch_data[37]);
    println!("  Op2 output level byte [58]: {} (should be 0)", patch_data[58]);
    println!("  Op3 output level byte [79]: {} (should be 0)", patch_data[79]);
    println!("  Op4 output level byte [100]: {} (should be 0)", patch_data[100]);
    println!("  Op5 output level byte [121]: {} (should be 0)", patch_data[121]);

    // Print operator configurations to verify only Op0 is active
    for (i, op) in patch_0.operators.iter().enumerate() {
        println!("Op{}: output_level={}, rates=[{},{},{},{}], levels=[{},{},{},{}]",
            i, op.output_level,
            op.rates.attack, op.rates.decay1, op.rates.decay2, op.rates.release,
            op.levels.attack, op.levels.decay1, op.levels.decay2, op.levels.release);

        let is_silent = op.output_level == 0 ||
                       (op.levels.attack == 0 && op.levels.decay1 == 0 && op.levels.decay2 == 0);
        println!("Op{} effectively silent: {}", i, is_silent);
    }

    // Create synth and load patch
    let mut synth = Dx7Synth::new(44100.0, 1.0);
    synth.load_patch(patch_0.clone())?;

    // Generate audio for C3 (MIDI note 48)
    let samples = synth.render_note(48, 127, 0.5)?; // 500ms at max velocity

    // Basic audio checks
    let non_zero_samples = samples.iter().filter(|&&x| x.abs() > 1e-8).count();
    println!("Generated {} samples, {} non-zero", samples.len(), non_zero_samples);

    assert!(non_zero_samples > 0, "Should generate non-zero audio for active operator");

    // RMS and peak analysis
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
    let peak = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("RMS: {:.6}, Peak: {:.6}", rms, peak);
    assert!(rms > 1e-6, "RMS should be significant for audible sine wave");

    // FFT Analysis to check for pure sine wave
    // Use a power-of-2 sample size for FFT
    let fft_size = 8192.min(samples.len());

    // Use middle portion to avoid attack/release artifacts
    let start_idx = samples.len() / 4;
    let end_idx = start_idx + fft_size;
    if end_idx > samples.len() {
        panic!("Not enough samples for FFT analysis");
    }
    let fft_samples = &samples[start_idx..end_idx];

    // Perform basic DFT analysis
    let sample_rate = 44100.0f32;
    let c3_freq = 130.81f32; // C3 frequency in Hz

    // Calculate magnitude spectrum using DFT
    let mut magnitudes = vec![0.0f32; fft_size / 2];

    for k in 0..fft_size/2 {
        let mut real = 0.0f32;
        let mut imag = 0.0f32;

        for n in 0..fft_size {
            let angle = -2.0 * std::f32::consts::PI * (k as f32) * (n as f32) / (fft_size as f32);
            real += fft_samples[n] * angle.cos();
            imag += fft_samples[n] * angle.sin();
        }

        magnitudes[k] = (real * real + imag * imag).sqrt();
    }

    // Find the frequency bin with maximum energy
    let mut max_bin = 0;
    let mut max_magnitude = 0.0f32;

    for (i, &mag) in magnitudes.iter().enumerate().skip(1) { // Skip DC component
        if mag > max_magnitude {
            max_magnitude = mag;
            max_bin = i;
        }
    }

    let fundamental_freq = (max_bin as f32) * sample_rate / (fft_size as f32);
    println!("Dominant frequency: {:.2} Hz (expected ~{:.2} Hz for C3)", fundamental_freq, c3_freq);

    // Assert fundamental frequency is close to C3
    let freq_tolerance = 5.0; // Tight tolerance for pure sine
    assert!((fundamental_freq - c3_freq).abs() < freq_tolerance,
        "Fundamental frequency {:.2} Hz should be close to C3 frequency {:.2} Hz (tolerance ±{} Hz)",
        fundamental_freq, c3_freq, freq_tolerance);

    // Calculate harmonic content - check that fundamental dominates
    let total_energy: f32 = magnitudes.iter().skip(1).map(|&x| x * x).sum();
    let fundamental_energy = magnitudes[max_bin] * magnitudes[max_bin];
    let fundamental_ratio = fundamental_energy / total_energy;

    println!("Fundamental energy ratio: {:.3} (should be > 0.9 for pure sine)", fundamental_ratio);

    // Assert that fundamental contains most of the energy (> 90% for pure sine)
    assert!(fundamental_ratio > 0.9,
        "Fundamental should contain >90% of energy for pure sine wave, got {:.3}",
        fundamental_ratio);

    // Check for significant harmonics - none should be more than 5% of fundamental
    let harmonic_threshold = 0.05 * magnitudes[max_bin];
    let mut significant_harmonics = Vec::new();

    for (i, &mag) in magnitudes.iter().enumerate().skip(1) {
        if i != max_bin && mag > harmonic_threshold {
            let freq = (i as f32) * sample_rate / (fft_size as f32);
            significant_harmonics.push((freq, mag / magnitudes[max_bin]));
        }
    }

    if !significant_harmonics.is_empty() {
        println!("Significant harmonics found:");
        for (freq, ratio) in &significant_harmonics {
            println!("  {:.2} Hz: {:.1}% of fundamental", freq, ratio * 100.0);
        }
    }

    // Assert no significant harmonics (should be pure sine)
    assert!(significant_harmonics.len() <= 1, // Allow one small harmonic due to digital artifacts
        "Pure sine wave should have no significant harmonics, found {} harmonics above 5% threshold",
        significant_harmonics.len());

    if significant_harmonics.len() == 1 {
        assert!(significant_harmonics[0].1 < 0.1, // Max 10% of fundamental
            "Largest harmonic should be <10% of fundamental, got {:.1}%",
            significant_harmonics[0].1 * 100.0);
    }

    println!("✅ Pure sine wave test passed: C3 at {:.2} Hz with {:.1}% fundamental energy",
             fundamental_freq, fundamental_ratio * 100.0);

    Ok(())
}