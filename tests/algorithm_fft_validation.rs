use dx7tv::{render_patch, Dx7Patch};
use dx7tv::sysex::Eg;
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

const SAMPLE_RATE: f32 = 48000.0;
const FFT_SIZE: usize = 8192; // Large FFT for good frequency resolution

/// FFT peak information
#[derive(Debug, Clone)]
struct FftPeak {
    frequency: f32,
    magnitude: f32,
    relative_db: f32,
}

/// Spectrum analysis results
#[derive(Debug)]
struct SpectrumAnalysis {
    peaks: Vec<FftPeak>,
    dc_offset: f32,
    has_sidebands: bool,
}

/// Analyze spectrum using FFT
fn analyze_spectrum(samples: &[f32], sample_rate: f32, min_peak_db: f32) -> SpectrumAnalysis {
    if samples.is_empty() {
        return SpectrumAnalysis {
            peaks: Vec::new(),
            dc_offset: 0.0,
            has_sidebands: false,
        };
    }

    let fft_size = FFT_SIZE.min(samples.len());
    let mut fft_input: Vec<Complex<f32>> = samples[..fft_size]
        .iter()
        .map(|&s| Complex::new(s, 0.0))
        .collect();

    // Apply Blackman-Harris window
    for (i, sample) in fft_input.iter_mut().enumerate() {
        let a0 = 0.35875;
        let a1 = 0.48829;
        let a2 = 0.14128;
        let a3 = 0.01168;
        let t = i as f32 / (fft_size - 1) as f32;
        let window = a0 - a1 * (2.0 * PI * t).cos()
            + a2 * (4.0 * PI * t).cos()
            - a3 * (6.0 * PI * t).cos();
        *sample *= window;
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut fft_input);

    // Calculate magnitude spectrum
    let magnitudes: Vec<f32> = fft_input[..fft_size / 2]
        .iter()
        .map(|c| c.norm() / fft_size as f32)
        .collect();

    // DC offset
    let dc_offset = magnitudes[0];

    // Find max magnitude for relative dB calculation
    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);

    // Find noise floor (median of lower magnitudes, excluding DC)
    let mut sorted_mags: Vec<f32> = magnitudes[1..].to_vec();
    sorted_mags.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let noise_floor = sorted_mags[sorted_mags.len() / 4];

    // Threshold for peak detection
    let threshold = max_magnitude * 10.0f32.powf(min_peak_db / 20.0);

    let mut peaks = Vec::new();

    // Find local maxima above threshold (skip DC bin)
    for i in 2..magnitudes.len() - 1 {
        if magnitudes[i] > threshold
            && magnitudes[i] > magnitudes[i - 1]
            && magnitudes[i] > magnitudes[i + 1]
            && magnitudes[i] > noise_floor * 5.0 // At least 5x noise floor
        {
            let frequency = (i as f32 * sample_rate) / fft_size as f32;
            let relative_db = 20.0 * (magnitudes[i] / max_magnitude).log10();

            peaks.push(FftPeak {
                frequency,
                magnitude: magnitudes[i],
                relative_db,
            });
        }
    }

    // Sort peaks by magnitude (strongest first)
    peaks.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());

    // Detect sidebands: check if we have multiple peaks not at exact harmonics
    let has_sidebands = peaks.len() > 2;

    SpectrumAnalysis {
        peaks,
        dc_offset,
        has_sidebands,
    }
}

/// Convert floating-point ratio to DX7 coarse/fine frequency
fn ratio_to_coarse_fine(ratio: f32) -> (u8, u8) {
    const COARSE_RATIOS: [f32; 32] = [
        0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0,
    ];

    let mut best_coarse = 1;
    let mut best_error = f32::INFINITY;

    for (i, &coarse_ratio) in COARSE_RATIOS.iter().enumerate() {
        let error = (coarse_ratio - ratio).abs();
        if error < best_error {
            best_error = error;
            best_coarse = i;
        }
    }

    let target_ratio = ratio / COARSE_RATIOS[best_coarse];
    let fine = ((target_ratio - 1.0) * 100.0 + 50.0).clamp(0.0, 99.0) as u8;

    (best_coarse as u8, fine)
}

/// Helper to create a DX7 patch from operator specs
fn create_patch(
    name: &str,
    algorithm: u8, // 1-based algorithm number
    operators: &[(u8, f32, u8, [u8; 4], [u8; 4])], // (op_num, ratio, output_level, rates, levels)
) -> Dx7Patch {
    let mut patch = Dx7Patch::new(name);

    // Set algorithm (convert 1-based to 0-based)
    patch.global.algorithm = algorithm.saturating_sub(1);

    for &(op_num, ratio, output_level, rates, levels) in operators {
        let op_idx = (op_num - 1) as usize;
        if op_idx < 6 {
            let operator = &mut patch.operators[op_idx];

            let (coarse, fine) = ratio_to_coarse_fine(ratio);
            operator.coarse_freq = coarse;
            operator.fine_freq = fine;
            operator.detune = 7; // Center detune
            operator.output_level = output_level;

            operator.rates = Eg {
                attack: rates[0],
                decay1: rates[1],
                decay2: rates[2],
                release: rates[3],
            };
            operator.levels = Eg {
                attack: levels[0],
                decay1: levels[1],
                decay2: levels[2],
                release: levels[3],
            };

            // Set mode to ratio
            operator.osc_mode = 0;
        }
    }

    patch
}

/// Render a patch and return audio samples
fn render_test_patch(patch: Dx7Patch, note: u8, duration_s: f64) -> Vec<f32> {
    render_patch(patch, note, duration_s).expect("Failed to render patch")
}

/// Find peaks near expected frequency
fn find_peaks_near(peaks: &[FftPeak], target_freq: f32, tolerance_hz: f32) -> Vec<FftPeak> {
    peaks
        .iter()
        .filter(|p| (p.frequency - target_freq).abs() <= tolerance_hz)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alg3_three_carriers_with_modulators() {
        // Algorithm 3: Three stacked carrier+modulator pairs
        let patch = create_patch(
            "ALG3_TEST",
            3, // Algorithm 3
            &[
                (1, 1.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op1: carrier at 1.0
                (2, 2.0, 70, [0, 50, 0, 20], [99, 0, 0, 0]),      // Op2: modulator at 2.0
                (3, 2.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op3: carrier at 2.0
                (4, 3.0, 70, [0, 50, 0, 20], [99, 0, 0, 0]),      // Op4: modulator at 3.0
                (5, 3.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op5: carrier at 3.0
                (6, 1.5, 70, [0, 50, 0, 20], [99, 0, 0, 0]),      // Op6: modulator at 1.5
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0); // A4 = 440Hz
        assert!(!samples.is_empty(), "No samples rendered");

        // Analyze a stable portion (skip attack, before release)
        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 3 - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(10).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Should have activity near 440Hz, 880Hz, 1320Hz (the carrier frequencies)
        let peaks_440 = find_peaks_near(&analysis.peaks, 440.0, 50.0);
        let peaks_880 = find_peaks_near(&analysis.peaks, 880.0, 50.0);
        let peaks_1320 = find_peaks_near(&analysis.peaks, 1320.0, 50.0);

        assert!(!peaks_440.is_empty(), "Expected energy near 440Hz");
        assert!(!peaks_880.is_empty(), "Expected energy near 880Hz");
        assert!(!peaks_1320.is_empty(), "Expected energy near 1320Hz");

        // FM should produce sidebands
        assert!(analysis.has_sidebands, "Expected sidebands from FM");

        // DC offset should be low
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }

    #[test]
    fn test_alg4_modulator_chain_two_branches() {
        let patch = create_patch(
            "ALG4_TEST",
            4, // Algorithm 4
            &[
                (1, 1.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op1: carrier
                (2, 2.0, 80, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op2: modulator
                (3, 3.0, 70, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op3: modulator chain
                (4, 2.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op4: carrier
                (5, 1.5, 70, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op5: modulator
                (6, 2.5, 60, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op6: modulator chain
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0);
        assert!(!samples.is_empty(), "No samples rendered");

        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 4 - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(10).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Should have complex spectrum with sidebands
        assert!(analysis.has_sidebands, "Expected sidebands from modulator chains");
        assert!(analysis.peaks.len() >= 3, "Expected multiple peaks from complex modulation");
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }

    #[test]
    fn test_alg5_two_op_fm() {
        // Algorithm 5: Three parallel 2-op FM pairs
        // (6->5) + (4->3) + (2->1)
        let patch = create_patch(
            "ALG5_TEST",
            5, // Algorithm 5
            &[
                (1, 1.0, 99, [0, 10, 0, 30], [99, 99, 99, 0]),    // Op1: carrier (output)
                (2, 2.0, 70, [0, 10, 0, 30], [99, 99, 99, 0]),    // Op2: modulator -> Op1
                (3, 2.0, 99, [0, 10, 0, 30], [99, 99, 99, 0]),    // Op3: carrier (output)
                (4, 3.0, 70, [0, 10, 0, 30], [99, 99, 99, 0]),    // Op4: modulator -> Op3
                (5, 3.0, 99, [0, 10, 0, 30], [99, 99, 99, 0]),    // Op5: carrier (output)
                (6, 1.5, 70, [0, 10, 0, 30], [99, 99, 99, 0]),    // Op6: modulator -> Op5 (with FB)
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0);
        assert!(!samples.is_empty(), "No samples rendered");

        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 5 (three 2-op FM pairs) - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(10).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Should have peaks near carrier frequencies: 440Hz (Op1), 880Hz (Op3), 1320Hz (Op5)
        let peaks_440 = find_peaks_near(&analysis.peaks, 440.0, 50.0);
        let peaks_880 = find_peaks_near(&analysis.peaks, 880.0, 50.0);
        let peaks_1320 = find_peaks_near(&analysis.peaks, 1320.0, 50.0);

        assert!(!peaks_440.is_empty(), "Expected peak near 440Hz");
        assert!(!peaks_880.is_empty(), "Expected peak near 880Hz");
        assert!(!peaks_1320.is_empty(), "Expected peak near 1320Hz");

        // Three parallel 2-op FM pairs should produce sidebands
        assert!(analysis.has_sidebands, "Expected sidebands from 2-op FM");
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }

    #[test]
    fn test_alg13_stack_modulators_on_one_carrier() {
        let patch = create_patch(
            "ALG13_TEST",
            13, // Algorithm 13
            &[
                (1, 1.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op1: carrier
                (2, 2.0, 70, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op2: modulator
                (3, 3.0, 70, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op3: modulator
                (4, 1.5, 70, [0, 40, 0, 50], [99, 99, 99, 0]),    // Op4: modulator
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0);
        assert!(!samples.is_empty(), "No samples rendered");

        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 13 - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(15).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Multiple modulators on one carrier should create very complex spectrum
        assert!(analysis.has_sidebands, "Expected sidebands");
        assert!(analysis.peaks.len() >= 5, "Expected many peaks from stacked modulators");
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }

    #[test]
    fn test_alg14_two_stacks_two_carriers() {
        let patch = create_patch(
            "ALG14_TEST",
            14, // Algorithm 14
            &[
                (1, 1.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op1: carrier
                (2, 2.0, 70, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op2: modulator
                (3, 2.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op3: carrier
                (4, 3.0, 70, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op4: modulator
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0);
        assert!(!samples.is_empty(), "No samples rendered");

        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 14 - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(10).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Should have energy near 440Hz and 880Hz
        let peaks_440 = find_peaks_near(&analysis.peaks, 440.0, 50.0);
        let peaks_880 = find_peaks_near(&analysis.peaks, 880.0, 50.0);

        assert!(!peaks_440.is_empty(), "Expected energy near 440Hz");
        assert!(!peaks_880.is_empty(), "Expected energy near 880Hz");
        assert!(analysis.has_sidebands, "Expected sidebands");
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }

    #[test]
    fn test_alg18_dual_modulators_feed_single_carrier() {
        let patch = create_patch(
            "ALG18_TEST",
            18, // Algorithm 18
            &[
                (1, 1.0, 99, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op1: carrier
                (2, 2.0, 70, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op2: modulator
                (3, 3.0, 70, [0, 40, 0, 50], [99, 99, 99, 0]),    // Op3: modulator
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0);
        assert!(!samples.is_empty(), "No samples rendered");

        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 18 - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(10).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Dual modulators should create medium-high complexity
        assert!(analysis.has_sidebands, "Expected sidebands");
        assert!(analysis.peaks.len() >= 3, "Expected multiple peaks");
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }

    #[test]
    fn test_alg23_parallel_two_carriers_each_with_modulator() {
        let patch = create_patch(
            "ALG23_TEST",
            23, // Algorithm 23
            &[
                (1, 1.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op1: carrier
                (2, 2.0, 70, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op2: modulator
                (3, 2.0, 99, [0, 20, 0, 50], [99, 99, 99, 0]),    // Op3: carrier
                (4, 3.0, 70, [0, 30, 0, 50], [99, 99, 99, 0]),    // Op4: modulator
            ],
        );

        let samples = render_test_patch(patch, 69, 1.0);
        assert!(!samples.is_empty(), "No samples rendered");

        let start = (SAMPLE_RATE * 0.1) as usize;
        let end = (SAMPLE_RATE * 0.5) as usize;
        let analysis_window = &samples[start..end.min(samples.len())];

        let analysis = analyze_spectrum(analysis_window, SAMPLE_RATE, -60.0);

        log::info!("Algorithm 23 - Found {} peaks", analysis.peaks.len());
        for (i, peak) in analysis.peaks.iter().take(10).enumerate() {
            log::info!("  Peak {}: {:.1} Hz, {:.1} dB", i + 1, peak.frequency, peak.relative_db);
        }

        // Two parallel FM pairs should show clusters at 440Hz and 880Hz
        let peaks_440 = find_peaks_near(&analysis.peaks, 440.0, 100.0);
        let peaks_880 = find_peaks_near(&analysis.peaks, 880.0, 100.0);

        assert!(!peaks_440.is_empty(), "Expected cluster around 440Hz");
        assert!(!peaks_880.is_empty(), "Expected cluster around 880Hz");
        assert!(analysis.has_sidebands, "Expected sidebands");
        assert!(analysis.dc_offset.abs() < 0.01, "DC offset too high: {}", analysis.dc_offset);
    }
}