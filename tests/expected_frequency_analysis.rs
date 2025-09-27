use dx7tv::sysex::Dx7Patch;
use dx7tv::synth::Dx7Synth;
use rustfft::{FftPlanner, num_complex::Complex};

/// Test to verify the expected frequency content you specified

#[test]
fn test_expected_frequency_spectrum() {
    let _ = env_logger::try_init();

    println!("=== Expected Frequency Content Analysis ===");

    // Use the actual star1-fast-decay.syx patch to test
    let patches = dx7tv::sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read star1-fast-decay.syx");
    let patch = patches.get(0).expect("No patches found").clone();

    println!("Testing patch: '{}'", patch.name);
    println!("Algorithm: {}", patch.global.algorithm + 1);

    let mut synth = Dx7Synth::new(44100.0, 2.0);
    synth.load_patch(patch).expect("Failed to load patch");

    // Generate audio for analysis
    let samples = synth.render_note(60, 100, 1.0).expect("Failed to render note");
    println!("Generated {} samples for analysis", samples.len());

    // Perform FFT analysis
    let spectrum = analyze_spectrum(&samples);

    // Expected frequency ranges with significant energy
    let expected_ranges = vec![
        (60.0, 65.0, "~62.5 Hz"),
        (125.0, 135.0, "~130 Hz"),
        (200.0, 210.0, "~204 Hz"),
        (300.0, 600.0, "midrange 300-600 Hz"),
        (2510.0, 4000.0, "high frequency 2.51k-4k Hz"),
    ];

    println!("\n=== Frequency Range Analysis ===");

    for (min_freq, max_freq, description) in &expected_ranges {
        let energy = calculate_energy_in_range(&spectrum, *min_freq, *max_freq, 44100.0);
        let peak_freq_mag = find_peak_in_range(&spectrum, *min_freq, *max_freq, 44100.0);

        println!("Range {}: Energy = {:.2}, Peak = {:?}",
                description, energy, peak_freq_mag);

        // Check if this range has significant energy
        if energy < 100.0 {
            println!("  ⚠️  LOW ENERGY in expected range {}", description);
        } else {
            println!("  ✅ Good energy in range {}", description);
        }
    }

    // Find the actual top frequency components
    println!("\n=== Actual Top Frequency Components ===");
    let all_peaks = find_all_peaks(&spectrum, 44100.0);

    println!("Top 15 frequency peaks:");
    for (i, (freq, mag)) in all_peaks.iter().take(15).enumerate() {
        println!("  {:2}. {:7.2} Hz: magnitude {:8.1}", i + 1, freq, mag);
    }

    // Analyze frequency distribution
    let total_energy = calculate_total_energy(&spectrum);
    let low_freq_energy = calculate_energy_in_range(&spectrum, 0.0, 100.0, 44100.0);
    let mid_freq_energy = calculate_energy_in_range(&spectrum, 100.0, 1000.0, 44100.0);
    let high_freq_energy = calculate_energy_in_range(&spectrum, 1000.0, 10000.0, 44100.0);

    println!("\n=== Energy Distribution ===");
    println!("Total energy: {:.1}", total_energy);
    println!("Low freq (0-100 Hz): {:.1} ({:.1}%)", low_freq_energy, (low_freq_energy/total_energy)*100.0);
    println!("Mid freq (100-1000 Hz): {:.1} ({:.1}%)", mid_freq_energy, (mid_freq_energy/total_energy)*100.0);
    println!("High freq (1000-10000 Hz): {:.1} ({:.1}%)", high_freq_energy, (high_freq_energy/total_energy)*100.0);

    // Test assertions based on expected spectrum
    assert!(
        calculate_energy_in_range(&spectrum, 60.0, 65.0, 44100.0) > 50.0,
        "Should have significant energy around 62.5 Hz"
    );

    assert!(
        calculate_energy_in_range(&spectrum, 125.0, 135.0, 44100.0) > 50.0,
        "Should have significant energy around 130 Hz"
    );

    assert!(
        calculate_energy_in_range(&spectrum, 200.0, 210.0, 44100.0) > 50.0,
        "Should have significant energy around 204 Hz"
    );

    assert!(
        calculate_energy_in_range(&spectrum, 300.0, 600.0, 44100.0) > 100.0,
        "Should have significant energy in midrange 300-600 Hz"
    );

    assert!(
        calculate_energy_in_range(&spectrum, 2510.0, 4000.0, 44100.0) > 50.0,
        "Should have significant energy in 2.51k-4k Hz range"
    );

    // Check that we don't have excessive low-frequency noise
    let low_freq_ratio = low_freq_energy / total_energy;
    assert!(
        low_freq_ratio < 0.7,
        "Should not be dominated by low frequencies: {:.1}% of energy is below 100Hz",
        low_freq_ratio * 100.0
    );

    println!("\n✅ Expected frequency analysis completed");
}

fn analyze_spectrum(samples: &[f32]) -> Vec<Complex<f64>> {
    let mut planner = FftPlanner::new();
    let fft_size = samples.len().next_power_of_two().min(16384); // Larger FFT for better frequency resolution
    let fft = planner.plan_fft_forward(fft_size);

    let mut buffer: Vec<Complex<f64>> = samples.iter()
        .take(fft_size)
        .map(|&x| Complex::new(x as f64, 0.0))
        .collect();

    buffer.resize(fft_size, Complex::new(0.0, 0.0));

    // Apply Hann window
    for (i, sample) in buffer.iter_mut().enumerate() {
        let window = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (fft_size - 1) as f64).cos());
        *sample *= window;
    }

    fft.process(&mut buffer);
    buffer
}

fn calculate_energy_in_range(spectrum: &[Complex<f64>], min_freq: f64, max_freq: f64, sample_rate: f64) -> f64 {
    let fft_size = spectrum.len();
    let freq_resolution = sample_rate / fft_size as f64;
    let min_bin = (min_freq / freq_resolution) as usize;
    let max_bin = (max_freq / freq_resolution) as usize;

    spectrum.iter()
        .enumerate()
        .skip(min_bin)
        .take(max_bin - min_bin)
        .map(|(_, c)| c.norm_sqr())
        .sum()
}

fn find_peak_in_range(spectrum: &[Complex<f64>], min_freq: f64, max_freq: f64, sample_rate: f64) -> Option<(f64, f64)> {
    let fft_size = spectrum.len();
    let freq_resolution = sample_rate / fft_size as f64;
    let min_bin = (min_freq / freq_resolution) as usize;
    let max_bin = (max_freq / freq_resolution) as usize;

    let mut peak_bin = min_bin;
    let mut peak_magnitude = 0.0;

    for i in min_bin..max_bin.min(fft_size / 2) {
        let magnitude = spectrum[i].norm();
        if magnitude > peak_magnitude {
            peak_magnitude = magnitude;
            peak_bin = i;
        }
    }

    if peak_magnitude > 1.0 {
        Some((peak_bin as f64 * freq_resolution, peak_magnitude))
    } else {
        None
    }
}

fn find_all_peaks(spectrum: &[Complex<f64>], sample_rate: f64) -> Vec<(f64, f64)> {
    let fft_size = spectrum.len();
    let freq_resolution = sample_rate / fft_size as f64;
    let mut peaks = Vec::new();

    for i in 1..(fft_size / 2 - 1) {
        let frequency = i as f64 * freq_resolution;
        let magnitude = spectrum[i].norm();

        // Peak detection: current bin is higher than neighbors
        let is_peak = magnitude > spectrum[i - 1].norm()
            && magnitude > spectrum[i + 1].norm()
            && magnitude > 10.0; // Minimum threshold

        if is_peak {
            peaks.push((frequency, magnitude));
        }
    }

    // Sort by magnitude (descending)
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    peaks
}

fn calculate_total_energy(spectrum: &[Complex<f64>]) -> f64 {
    spectrum.iter()
        .take(spectrum.len() / 2)
        .map(|c| c.norm_sqr())
        .sum()
}