use std::time::Duration;

use rustfft::{num_complex::Complex, FftPlanner};

mod common;
use common::generate_samples;

use dx7::fm::patch::{OpEnvelope, Operator, Patch};

/// Helper to create a simple operator with specified ratio and envelope
fn create_operator(ratio_coarse: u8, ratio_fine: u8, output_level: u8) -> Operator {
    Operator {
        envelope: OpEnvelope {
            rate: [99, 99, 99, 99],
            level: [99, 99, 99, 0],
        },
        level: output_level,
        coarse: ratio_coarse,
        fine: ratio_fine,
        mode: 0, // ratio mode
        ..Operator::default()
    }
}

/// Helper to convert frequency ratio to coarse/fine values
/// For simple integer ratios, we use coarse; for fractional ones like 1.5, we use fine
fn ratio_to_coarse_fine(ratio: f32) -> (u8, u8) {
    // Approximate with fine tuning from nearest integer
    let coarse = ratio.floor() as u8;
    let fine = ((ratio - coarse as f32) * 100.0) as u8;
    (coarse, fine)
}

/// Perform FFT and find peaks above a threshold
fn analyze_spectrum(samples: &[f32], sample_rate: u32) -> Vec<(f32, f32)> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(samples.len());

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();

    fft.process(&mut buffer);

    // Convert to magnitude spectrum (only positive frequencies)
    let bin_width = sample_rate as f32 / samples.len() as f32;

    let magnitudes: Vec<(f32, f32)> = buffer
        .iter()
        .take(buffer.len() / 2)
        .enumerate()
        .map(|(i, c)| {
            let freq = i as f32 * bin_width;
            let mag = (c.re * c.re + c.im * c.im).sqrt();
            (freq, mag)
        })
        .collect();

    magnitudes
}

/// Find peaks in the spectrum above a relative threshold
fn find_peaks(spectrum: &[(f32, f32)], relative_threshold_db: f32) -> Vec<(f32, f32)> {
    let max_mag = spectrum
        .iter()
        .map(|(_, mag)| mag)
        .fold(0.0f32, |a, &b| a.max(b));

    let threshold = max_mag * 10.0_f32.powf(relative_threshold_db / 20.0);

    let mut peaks = Vec::new();
    for i in 1..spectrum.len() - 1 {
        let (freq, mag) = spectrum[i];
        if mag > threshold && mag > spectrum[i - 1].1 && mag > spectrum[i + 1].1 {
            peaks.push((freq, mag));
        }
    }

    peaks
}

/// Calculate DC offset
fn calculate_dc_offset(samples: &[f32]) -> f32 {
    samples.iter().sum::<f32>() / samples.len() as f32
}

/// Check if there are significant sidebands around a fundamental frequency
fn has_sidebands(spectrum: &[(f32, f32)], fundamental: f32, tolerance_hz: f32) -> bool {
    let peaks = find_peaks(spectrum, -20.0);

    // Count peaks near the fundamental (within tolerance)
    let near_fundamental = peaks
        .iter()
        .filter(|(f, _)| (*f - fundamental).abs() < tolerance_hz)
        .count();

    // Count peaks that are sidebands (not exactly at fundamental or harmonics)
    let sideband_count = peaks
        .iter()
        .filter(|(f, _)| {
            let harmonic_number = f / fundamental;
            let distance_from_harmonic = (harmonic_number - harmonic_number.round()).abs();
            distance_from_harmonic > 0.1 && distance_from_harmonic < 0.9
        })
        .count();

    sideband_count > 0
}

#[test]
fn test_alg3_three_carriers_with_modulators() {
    // Algorithm 3 (0-indexed: 2): three pairs of modulator->carrier
    let mut patch = Patch::default();
    patch.algorithm = 2; // 0-indexed

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(2, create_operator(c, f, 70));

    // Op 3: carrier, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(3, create_operator(c, f, 99));

    // Op 4: modulator, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(4, create_operator(c, f, 70));

    // Op 5: carrier, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(5, create_operator(c, f, 99));

    // Op 6: modulator, ratio 1.5
    let (c, f) = ratio_to_coarse_fine(1.5);
    patch.set_op(6, create_operator(c, f, 70));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);
    let peaks = find_peaks(&spectrum, -20.0);

    // Should have energy near 437Hz, 874Hz, 1311Hz (A4 and harmonics with this MIDI mapping)
    let has_437 = peaks.iter().any(|(f, _)| (*f - 437.0).abs() < 10.0);
    let has_874 = peaks.iter().any(|(f, _)| (*f - 874.0).abs() < 10.0);
    let has_1311 = peaks.iter().any(|(f, _)| (*f - 1311.0).abs() < 10.0);

    assert!(has_437, "Expected peak near 437Hz (A4)");
    assert!(has_874, "Expected peak near 874Hz (A5)");
    assert!(has_1311, "Expected peak near 1311Hz");

    // Should have sidebands
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands around fundamental"
    );
}

#[test]
fn test_alg4_modulator_chain_two_branches() {
    // Algorithm 4 (0-indexed: 3)
    let mut patch = Patch::default();
    patch.algorithm = 3;

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(2, create_operator(c, f, 80));

    // Op 3: modulator, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(3, create_operator(c, f, 70));

    // Op 4: carrier, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(4, create_operator(c, f, 99));

    // Op 5: modulator, ratio 1.5
    let (c, f) = ratio_to_coarse_fine(1.5);
    patch.set_op(5, create_operator(c, f, 70));

    // Op 6: modulator, ratio 2.5
    let (c, f) = ratio_to_coarse_fine(2.5);
    patch.set_op(6, create_operator(c, f, 60));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);

    // Should have sidebands due to complex modulation chains
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands from complex modulation"
    );
}

#[test]
fn test_alg5_two_op_fm() {
    // Algorithm 5 (0-indexed: 4): classic 2-operator FM
    let mut patch = Patch::default();
    patch.algorithm = 4;

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(2, create_operator(c, f, 70));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);
    let peaks = find_peaks(&spectrum, -20.0);

    // Should have a peak near 437Hz (A4 with this MIDI mapping)
    let has_437 = peaks.iter().any(|(f, _)| (*f - 437.0).abs() < 10.0);
    assert!(has_437, "Expected peak near 437Hz (A4)");

    // Should have sidebands from 2-op FM
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands from 2-operator FM"
    );
}

#[test]
fn test_alg13_stack_modulators_on_one_carrier() {
    // Algorithm 13 (0-indexed: 12): multiple parallel modulators on single carrier
    let mut patch = Patch::default();
    patch.algorithm = 12;

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);

    // Op 3: modulator, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(3, create_operator(c, f, 70));

    // Op 4: modulator, ratio 1.5
    let (c, f) = ratio_to_coarse_fine(1.5);
    patch.set_op(4, create_operator(c, f, 70));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);
    let peaks = find_peaks(&spectrum, -20.0);

    // Should have very complex spectrum with many peaks
    assert!(
        peaks.len() > 5,
        "Expected complex spectrum with many peaks, found {}",
        peaks.len()
    );

    // Should have strong sidebands
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected strong sidebands from stacked modulators"
    );
}

#[test]
fn test_alg14_two_stacks_two_carriers() {
    // Algorithm 14 (0-indexed: 13): two parallel carrier/modulator stacks
    let mut patch = Patch::default();
    patch.algorithm = 13;

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(2, create_operator(c, f, 70));

    // Op 3: carrier, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(3, create_operator(c, f, 99));

    // Op 4: modulator, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(4, create_operator(c, f, 70));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);
    let peaks = find_peaks(&spectrum, -20.0);

    // Should have energy near both 440Hz and 880Hz
    let has_440 = peaks.iter().any(|(f, _)| (*f - 437.0).abs() < 10.0);
    let has_880 = peaks.iter().any(|(f, _)| (*f - 874.0).abs() < 10.0);

    assert!(has_440, "Expected peak near 440Hz");
    assert!(has_880, "Expected peak near 880Hz");

    // Should have sidebands
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands around fundamental"
    );
}

#[test]
fn test_alg18_dual_modulators_feed_single_carrier() {
    // Algorithm 18 (0-indexed: 17)
    let mut patch = Patch::default();
    patch.algorithm = 17;

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(2, create_operator(c, f, 70));

    // Op 3: modulator, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(3, create_operator(c, f, 70));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);

    // Should have sidebands from dual modulation
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands from dual modulators"
    );
}

#[test]
fn test_alg23_parallel_two_carriers_each_with_modulator() {
    // Algorithm 23 (0-indexed: 22)
    let mut patch = Patch::default();
    patch.algorithm = 22;

    // Op 1: carrier, ratio 1.0
    let (c, f) = ratio_to_coarse_fine(1.0);
    patch.set_op(1, create_operator(c, f, 99));

    // Op 2: modulator, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(2, create_operator(c, f, 70));

    // Op 3: carrier, ratio 2.0
    let (c, f) = ratio_to_coarse_fine(2.0);
    patch.set_op(3, create_operator(c, f, 99));

    // Op 4: modulator, ratio 3.0
    let (c, f) = ratio_to_coarse_fine(3.0);
    patch.set_op(4, create_operator(c, f, 70));

    let samples = generate_samples(patch, 69.0, 48000, Duration::from_millis(1000));

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Analyze spectrum
    let spectrum = analyze_spectrum(&samples, 48000);
    let peaks = find_peaks(&spectrum, -20.0);

    // Should have two main clusters around 440Hz and 880Hz
    let has_440 = peaks.iter().any(|(f, _)| (*f - 437.0).abs() < 10.0);
    let has_880 = peaks.iter().any(|(f, _)| (*f - 874.0).abs() < 10.0);

    assert!(has_440, "Expected peak near 440Hz");
    assert!(has_880, "Expected peak near 880Hz");

    // Should have sidebands
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands from parallel FM"
    );
}

#[test]
fn test_alg32_parallel_sines() {
    const SAMPLE_RATE: u32 = 48000;

    // Algorithm 23 (0-indexed: 22)
    let mut patch = Patch::default();
    patch.algorithm = 31;

    for idx in 1..=6 {
        let (c, f) = ratio_to_coarse_fine(idx as f32);
        patch.set_op(idx, create_operator(c, f, 99));
    }

    let desired_pitch = 437.0;
    let sample_pitch_increment = desired_pitch / SAMPLE_RATE as f32;

    for pitch in [48.0, 60.0, 69.0, 72.0, 81.0] {
        let samples = generate_samples(patch, pitch, SAMPLE_RATE, Duration::from_millis(1000));

        // Analyze spectrum
        let spectrum = analyze_spectrum(&samples, SAMPLE_RATE);
        let peaks = find_peaks(&spectrum, -20.0);

        println!("pitch: {}, peaks: {:?}", pitch, peaks);
    }

    /*

    // Check DC offset
    let dc_offset = calculate_dc_offset(&samples);
    assert!(dc_offset.abs() < 0.1, "DC offset too high: {}", dc_offset);

    // Should have two main clusters around 440Hz and 880Hz
    let has_440 = peaks.iter().any(|(f, _)| (*f - 437.0).abs() < 10.0);
    let has_880 = peaks.iter().any(|(f, _)| (*f - 874.0).abs() < 10.0);

    assert!(has_440, "Expected peak near 440Hz");
    assert!(has_880, "Expected peak near 880Hz");

    // Should have sidebands
    assert!(
        has_sidebands(&spectrum, 437.0, 50.0),
        "Expected sidebands from parallel FM"
    );
    */
}
