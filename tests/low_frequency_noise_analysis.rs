use dx7tv::sysex::Dx7Patch;
use dx7tv::synth::Dx7Synth;
use rustfft::{FftPlanner, num_complex::Complex};

/// Test to diagnose excessive low-frequency content causing noise perception

#[test]
fn test_dc_offset_and_low_frequency_analysis() {
    let _ = env_logger::try_init();

    println!("=== DC Offset and Low-Frequency Noise Analysis ===");

    // Test 1: Algorithm 4 (FM)
    let mut fm_patch = Dx7Patch::new("ALGO4_DC");
    fm_patch.global.algorithm = 3;

    // Simple 2-op FM setup
    fm_patch.operators[0].coarse_freq = 2;
    fm_patch.operators[0].output_level = 80;
    fm_patch.operators[0].rates.attack = 99;
    fm_patch.operators[0].levels.attack = 99;

    fm_patch.operators[1].coarse_freq = 1;
    fm_patch.operators[1].output_level = 90;
    fm_patch.operators[1].rates.attack = 99;
    fm_patch.operators[1].levels.attack = 99;

    for i in 2..6 {
        fm_patch.operators[i].output_level = 0;
    }

    // Test 2: Simple sine wave for comparison
    let mut sine_patch = Dx7Patch::new("SINE_DC");
    sine_patch.global.algorithm = 31; // Algorithm 32 (parallel)
    sine_patch.operators[0].coarse_freq = 1;
    sine_patch.operators[0].output_level = 90;
    sine_patch.operators[0].rates.attack = 99;
    sine_patch.operators[0].levels.attack = 99;

    for i in 1..6 {
        sine_patch.operators[i].output_level = 0;
    }

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Generate samples
    synth.load_patch(fm_patch).expect("Failed to load FM patch");
    let fm_samples = synth.render_note(60, 100, 0.5).expect("Failed to render FM");

    synth.load_patch(sine_patch).expect("Failed to load sine patch");
    let sine_samples = synth.render_note(60, 100, 0.5).expect("Failed to render sine");

    // Analyze DC offset
    println!("\n=== DC Offset Analysis ===");
    let fm_dc = calculate_dc_offset(&fm_samples);
    let sine_dc = calculate_dc_offset(&sine_samples);

    println!("FM DC offset: {:.8}", fm_dc);
    println!("Sine DC offset: {:.8}", sine_dc);

    // Analyze low-frequency content (< 50 Hz)
    println!("\n=== Low-Frequency Content Analysis ===");
    let fm_spectrum = analyze_spectrum(&fm_samples);
    let sine_spectrum = analyze_spectrum(&sine_samples);

    let fm_lf_energy = calculate_low_freq_energy(&fm_spectrum, 50.0, 44100.0);
    let sine_lf_energy = calculate_low_freq_energy(&sine_spectrum, 50.0, 44100.0);

    let fm_total_energy = calculate_total_energy(&fm_spectrum);
    let sine_total_energy = calculate_total_energy(&sine_spectrum);

    let fm_lf_ratio = fm_lf_energy / fm_total_energy;
    let sine_lf_ratio = sine_lf_energy / sine_total_energy;

    println!("FM low-freq (<50Hz) energy ratio: {:.2}%", fm_lf_ratio * 100.0);
    println!("Sine low-freq (<50Hz) energy ratio: {:.2}%", sine_lf_ratio * 100.0);

    // Analyze subsonic content (< 20 Hz)
    let fm_subsonic = calculate_low_freq_energy(&fm_spectrum, 20.0, 44100.0);
    let sine_subsonic = calculate_low_freq_energy(&sine_spectrum, 20.0, 44100.0);

    let fm_subsonic_ratio = fm_subsonic / fm_total_energy;
    let sine_subsonic_ratio = sine_subsonic / sine_total_energy;

    println!("FM subsonic (<20Hz) energy ratio: {:.2}%", fm_subsonic_ratio * 100.0);
    println!("Sine subsonic (<20Hz) energy ratio: {:.2}%", sine_subsonic_ratio * 100.0);

    // Find the dominant low-frequency components
    println!("\n=== Dominant Low-Frequency Components ===");
    let fm_lf_peaks = find_peaks_in_range(&fm_spectrum, 0.0, 100.0, 44100.0);
    let sine_lf_peaks = find_peaks_in_range(&sine_spectrum, 0.0, 100.0, 44100.0);

    println!("FM - Top 5 low-frequency peaks:");
    for (i, (freq, mag)) in fm_lf_peaks.iter().take(5).enumerate() {
        println!("  {}. {:.2} Hz: magnitude {:.2}", i + 1, freq, mag);
    }

    println!("Sine - Top 5 low-frequency peaks:");
    for (i, (freq, mag)) in sine_lf_peaks.iter().take(5).enumerate() {
        println!("  {}. {:.2} Hz: magnitude {:.2}", i + 1, freq, mag);
    }

    // Analyze sample statistics
    println!("\n=== Sample Statistics ===");
    let fm_stats = calculate_sample_stats(&fm_samples);
    let sine_stats = calculate_sample_stats(&sine_samples);

    println!("FM samples - Mean: {:.8}, StdDev: {:.6}, Min: {:.6}, Max: {:.6}",
             fm_stats.0, fm_stats.1, fm_stats.2, fm_stats.3);
    println!("Sine samples - Mean: {:.8}, StdDev: {:.6}, Min: {:.6}, Max: {:.6}",
             sine_stats.0, sine_stats.1, sine_stats.2, sine_stats.3);

    // Check for problematic characteristics
    println!("\n=== Problem Detection ===");

    // 1. Excessive DC offset
    if fm_dc.abs() > 0.01 {
        println!("⚠️  FM has significant DC offset: {:.6}", fm_dc);
    }

    // 2. Excessive low-frequency energy
    if fm_lf_ratio > 0.3 {
        println!("⚠️  FM has excessive low-frequency energy: {:.1}%", fm_lf_ratio * 100.0);
    }

    // 3. Excessive subsonic energy
    if fm_subsonic_ratio > 0.2 {
        println!("⚠️  FM has excessive subsonic energy: {:.1}%", fm_subsonic_ratio * 100.0);
    }

    // 4. Compare to sine wave
    if fm_lf_ratio > sine_lf_ratio * 5.0 {
        println!("⚠️  FM has {}x more low-frequency energy than sine wave", fm_lf_ratio / sine_lf_ratio);
    }

    // Test assertions
    assert!(
        fm_dc.abs() < 0.1,
        "FM should not have excessive DC offset: {:.6}",
        fm_dc
    );

    assert!(
        fm_lf_ratio < 0.5,
        "FM should not have excessive low-frequency energy: {:.1}%",
        fm_lf_ratio * 100.0
    );

    println!("✅ Low-frequency analysis completed");
}

fn calculate_dc_offset(samples: &[f32]) -> f64 {
    samples.iter().map(|&x| x as f64).sum::<f64>() / samples.len() as f64
}

fn analyze_spectrum(samples: &[f32]) -> Vec<Complex<f64>> {
    let mut planner = FftPlanner::new();
    let fft_size = samples.len().next_power_of_two().min(8192);
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

fn calculate_low_freq_energy(spectrum: &[Complex<f64>], max_freq: f64, sample_rate: f64) -> f64 {
    let fft_size = spectrum.len();
    let freq_resolution = sample_rate / fft_size as f64;
    let max_bin = (max_freq / freq_resolution) as usize;

    spectrum.iter()
        .take(max_bin.min(fft_size / 2))
        .map(|c| c.norm_sqr())
        .sum()
}

fn calculate_total_energy(spectrum: &[Complex<f64>]) -> f64 {
    spectrum.iter()
        .take(spectrum.len() / 2)
        .map(|c| c.norm_sqr())
        .sum()
}

fn find_peaks_in_range(spectrum: &[Complex<f64>], min_freq: f64, max_freq: f64, sample_rate: f64) -> Vec<(f64, f64)> {
    let fft_size = spectrum.len();
    let freq_resolution = sample_rate / fft_size as f64;
    let min_bin = (min_freq / freq_resolution) as usize;
    let max_bin = (max_freq / freq_resolution) as usize;

    let mut peaks = Vec::new();

    for i in min_bin..max_bin.min(fft_size / 2) {
        let frequency = i as f64 * freq_resolution;
        let magnitude = spectrum[i].norm();

        // Simple peak detection
        let is_peak = i > 0 && i < fft_size / 2 - 1
            && magnitude > spectrum[i - 1].norm()
            && magnitude > spectrum[i + 1].norm()
            && magnitude > 1.0; // Minimum threshold

        if is_peak {
            peaks.push((frequency, magnitude));
        }
    }

    // Sort by magnitude (descending)
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    peaks
}

fn calculate_sample_stats(samples: &[f32]) -> (f64, f64, f32, f32) {
    let mean = samples.iter().map(|&x| x as f64).sum::<f64>() / samples.len() as f64;

    let variance = samples.iter()
        .map(|&x| {
            let diff = x as f64 - mean;
            diff * diff
        })
        .sum::<f64>() / samples.len() as f64;

    let std_dev = variance.sqrt();
    let min = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    (mean, std_dev, min, max)
}