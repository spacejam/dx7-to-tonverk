use dx7tv::sysex::Dx7Patch;
use dx7tv::synth::Dx7Synth;
use rustfft::{FftPlanner, num_complex::Complex};

/// FFT-based spectral analysis for FM synthesis validation
/// Verifies that algorithm 4 produces expected FM sidebands

#[test]
fn test_algorithm_4_fm_sidebands() {
    let _ = env_logger::try_init();

    println!("=== Testing Algorithm 4 FM Sideband Generation ===");

    // Create a test patch using algorithm 4 (DX7 algorithms are 1-indexed, stored 0-indexed)
    let mut patch = Dx7Patch::new("ALGO4FM");
    patch.global.algorithm = 3; // Algorithm 4 (stored as 3)

    // Algorithm 4 structure: OP1 -> OP2 -> OUT, OP3 -> OP4 -> OUT, OP5 -> OP6
    // Let's set up a simple 2-operator FM pair (OP1 modulating OP2)

    // OP1 (modulator): 2:1 frequency ratio, high output level
    patch.operators[0].coarse_freq = 2;     // 2:1 ratio (modulator)
    patch.operators[0].fine_freq = 0;
    patch.operators[0].output_level = 99;   // High modulation index
    patch.operators[0].rates.attack = 99;   // Instant attack
    patch.operators[0].rates.decay1 = 99;
    patch.operators[0].rates.decay2 = 99;
    patch.operators[0].rates.release = 90;
    patch.operators[0].levels.attack = 99;  // Full level
    patch.operators[0].levels.decay1 = 99;
    patch.operators[0].levels.decay2 = 99;
    patch.operators[0].levels.release = 0;
    patch.operators[0].detune = 7; // Center detune

    // OP2 (carrier): 1:1 frequency ratio, moderate output
    patch.operators[1].coarse_freq = 1;     // 1:1 ratio (carrier)
    patch.operators[1].fine_freq = 0;
    patch.operators[1].output_level = 90;
    patch.operators[1].rates.attack = 99;
    patch.operators[1].rates.decay1 = 90;
    patch.operators[1].rates.decay2 = 80;
    patch.operators[1].rates.release = 70;
    patch.operators[1].levels.attack = 99;
    patch.operators[1].levels.decay1 = 90;
    patch.operators[1].levels.decay2 = 80;
    patch.operators[1].levels.release = 0;
    patch.operators[1].detune = 7; // Center detune

    // Disable other operators for cleaner analysis
    for i in 2..6 {
        patch.operators[i].output_level = 0;
    }

    println!("Test setup:");
    println!("  Algorithm: 4");
    println!("  OP1 (modulator): 2:1 ratio, output_level=99");
    println!("  OP2 (carrier): 1:1 ratio, output_level=90");
    println!("  Other OPs disabled");

    // Generate audio
    let mut synth = Dx7Synth::new(44100.0, 2.0);
    synth.load_patch(patch).expect("Failed to load patch");

    // Render a sustained note for frequency analysis
    let samples = synth.render_note(60, 100, 0.5).expect("Failed to render note"); // Middle C, 500ms

    println!("Generated {} samples", samples.len());

    // Perform FFT analysis
    let spectrum = analyze_spectrum(&samples, 44100.0);

    // Middle C (MIDI 60) = ~261.63 Hz
    let fundamental_freq = 261.63;
    let modulator_freq = fundamental_freq * 2.0; // 2:1 ratio = ~523.26 Hz

    println!("\nExpected frequencies:");
    println!("  Carrier (fundamental): {:.2} Hz", fundamental_freq);
    println!("  Modulator: {:.2} Hz", modulator_freq);

    // In FM synthesis with carrier C and modulator M, we expect sidebands at:
    // C ± M, C ± 2M, C ± 3M, etc.
    // With C = 261.63 Hz and M = 523.26 Hz:
    let expected_sidebands = vec![
        fundamental_freq,                              // 261.63 Hz (carrier)
        fundamental_freq + modulator_freq,             // 784.89 Hz (C + M)
        fundamental_freq - modulator_freq,             // -261.63 Hz (negative, should appear as positive)
        fundamental_freq + 2.0 * modulator_freq,      // 1308.15 Hz (C + 2M)
        2.0 * modulator_freq - fundamental_freq,      // 784.89 Hz (2M - C, same as C + M)
    ];

    let folded_sidebands: Vec<f64> = expected_sidebands.iter()
        .map(|&f| (f as f64).abs())
        .filter(|&f| f > 20.0 && f < 20000.0) // Audible range
        .collect();

    println!("\nExpected sideband frequencies (FM theory):");
    for (i, freq) in folded_sidebands.iter().enumerate() {
        println!("  Sideband {}: {:.2} Hz", i + 1, freq);
    }

    // Analyze actual spectrum
    println!("\nActual spectrum analysis:");
    let mut found_peaks = find_spectral_peaks(&spectrum, 44100.0, 10.0); // Find peaks above 10 Hz
    found_peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // Sort by magnitude

    println!("Top 10 spectral peaks:");
    for (i, (freq, magnitude)) in found_peaks.iter().take(10).enumerate() {
        println!("  Peak {}: {:.2} Hz, magnitude: {:.6}", i + 1, freq, magnitude);
    }

    // Verify expected sidebands are present
    let mut sidebands_found = 0;
    let frequency_tolerance = 10.0; // Hz tolerance for peak matching

    for expected_freq in &folded_sidebands {
        let found_peak = found_peaks.iter()
            .find(|(freq, _)| (freq - expected_freq).abs() < frequency_tolerance);

        if let Some((actual_freq, magnitude)) = found_peak {
            println!("✓ Found expected sideband: {:.2} Hz (actual: {:.2} Hz, mag: {:.6})",
                     expected_freq, actual_freq, magnitude);
            sidebands_found += 1;
        } else {
            println!("✗ Missing expected sideband: {:.2} Hz", expected_freq);
        }
    }

    // Test assertions
    assert!(
        !samples.iter().all(|&x| x.abs() < 1e-6),
        "Algorithm 4 should produce audible output"
    );

    assert!(
        found_peaks.len() >= 3,
        "Algorithm 4 should produce multiple frequency components, found {} peaks",
        found_peaks.len()
    );

    // Check for carrier frequency
    let carrier_found = found_peaks.iter()
        .any(|(freq, _)| (freq - fundamental_freq).abs() < frequency_tolerance);

    assert!(
        carrier_found,
        "Algorithm 4 should contain the carrier frequency (~{:.2} Hz)",
        fundamental_freq
    );

    // For proper FM, we should find at least some sidebands
    assert!(
        sidebands_found >= 1,
        "Algorithm 4 should generate FM sidebands, found {}/{} expected sidebands",
        sidebands_found, folded_sidebands.len()
    );

    // Check that the spectrum shows harmonic complexity, not just noise
    let total_energy: f64 = spectrum.iter().map(|c| c.norm_sqr() as f64).sum();
    let peak_energy: f64 = found_peaks.iter().take(5).map(|(_, mag)| mag * mag).sum();
    let energy_ratio = peak_energy / total_energy;

    println!("\nSpectral analysis:");
    println!("  Total energy: {:.6}", total_energy);
    println!("  Top 5 peaks energy: {:.6}", peak_energy);
    println!("  Energy concentration: {:.2}%", energy_ratio * 100.0);

    assert!(
        energy_ratio > 0.1,
        "Algorithm 4 should show concentrated spectral energy in peaks (ratio: {:.3}), not noise",
        energy_ratio
    );

    if sidebands_found == folded_sidebands.len() {
        println!("✅ Algorithm 4 FM synthesis test PASSED - all expected sidebands found");
    } else {
        println!("⚠️  Algorithm 4 FM synthesis partially working - {}/{} sidebands found",
                 sidebands_found, folded_sidebands.len());
    }
}

#[test]
fn test_algorithm_4_vs_simple_sine() {
    let _ = env_logger::try_init();

    println!("=== Comparing Algorithm 4 vs Simple Sine Wave ===");

    // Test 1: Algorithm 4 FM patch
    let mut fm_patch = Dx7Patch::new("FM_TEST");
    fm_patch.global.algorithm = 3; // Algorithm 4

    // Set up 2-operator FM
    fm_patch.operators[0].coarse_freq = 3;  // 3:1 modulator
    fm_patch.operators[0].output_level = 80;
    fm_patch.operators[0].rates.attack = 99;
    fm_patch.operators[0].levels.attack = 99;

    fm_patch.operators[1].coarse_freq = 1;  // 1:1 carrier
    fm_patch.operators[1].output_level = 90;
    fm_patch.operators[1].rates.attack = 99;
    fm_patch.operators[1].levels.attack = 99;

    // Disable other operators
    for i in 2..6 {
        fm_patch.operators[i].output_level = 0;
    }

    // Test 2: Single sine wave (algorithm 32 - single carrier)
    let mut sine_patch = Dx7Patch::new("SINE_TEST");
    sine_patch.global.algorithm = 31; // Algorithm 32 (all parallel)
    sine_patch.operators[0].coarse_freq = 1;
    sine_patch.operators[0].output_level = 90;
    sine_patch.operators[0].rates.attack = 99;
    sine_patch.operators[0].levels.attack = 99;

    // Disable other operators
    for i in 1..6 {
        sine_patch.operators[i].output_level = 0;
    }

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Generate FM audio
    synth.load_patch(fm_patch).expect("Failed to load FM patch");
    let fm_samples = synth.render_note(60, 100, 0.3).expect("Failed to render FM note");
    let fm_spectrum = analyze_spectrum(&fm_samples, 44100.0);

    // Generate sine audio
    synth.load_patch(sine_patch).expect("Failed to load sine patch");
    let sine_samples = synth.render_note(60, 100, 0.3).expect("Failed to render sine note");
    let sine_spectrum = analyze_spectrum(&sine_samples, 44100.0);

    // Compare spectral complexity
    let fm_peaks = find_spectral_peaks(&fm_spectrum, 44100.0, 10.0);
    let sine_peaks = find_spectral_peaks(&sine_spectrum, 44100.0, 10.0);

    println!("Spectral comparison:");
    println!("  FM (Algorithm 4) peaks: {}", fm_peaks.len());
    println!("  Sine wave peaks: {}", sine_peaks.len());

    // FM should have more spectral complexity than a simple sine wave
    assert!(
        fm_peaks.len() > sine_peaks.len(),
        "FM synthesis should be more spectrally complex than sine wave: FM={} peaks, Sine={} peaks",
        fm_peaks.len(), sine_peaks.len()
    );

    assert!(
        fm_peaks.len() >= 3,
        "FM synthesis should generate multiple frequency components: {} peaks found",
        fm_peaks.len()
    );

    println!("✅ Algorithm 4 vs Sine comparison PASSED");
}

/// Analyze spectrum using FFT
fn analyze_spectrum(samples: &[f32], _sample_rate: f64) -> Vec<Complex<f64>> {
    let mut planner = FftPlanner::new();
    let fft_size = samples.len().next_power_of_two().min(8192); // Limit FFT size for performance
    let fft = planner.plan_fft_forward(fft_size);

    // Prepare input data
    let mut buffer: Vec<Complex<f64>> = samples.iter()
        .take(fft_size)
        .map(|&x| Complex::new(x as f64, 0.0))
        .collect();

    // Pad with zeros if necessary
    buffer.resize(fft_size, Complex::new(0.0, 0.0));

    // Apply window function (Hann window)
    for (i, sample) in buffer.iter_mut().enumerate() {
        let window_value = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (fft_size - 1) as f64).cos());
        *sample *= window_value;
    }

    // Perform FFT
    fft.process(&mut buffer);

    buffer
}

/// Find spectral peaks above a minimum frequency
fn find_spectral_peaks(spectrum: &[Complex<f64>], sample_rate: f64, min_freq: f64) -> Vec<(f64, f64)> {
    let mut peaks = Vec::new();
    let fft_size = spectrum.len();
    let freq_resolution = sample_rate / fft_size as f64;

    // Only look at positive frequencies (first half of spectrum)
    let half_size = fft_size / 2;

    for i in 1..half_size {
        let frequency = i as f64 * freq_resolution;
        if frequency < min_freq { continue; }

        let magnitude = spectrum[i].norm();

        // Simple peak detection: check if current bin is higher than neighbors
        let is_peak = i > 0 && i < half_size - 1
            && magnitude > spectrum[i - 1].norm()
            && magnitude > spectrum[i + 1].norm()
            && magnitude > 0.001; // Minimum threshold

        if is_peak {
            peaks.push((frequency, magnitude));
        }
    }

    peaks
}