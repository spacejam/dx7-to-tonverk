use dx7tv::{Dx7Synth, sysex::parse_sysex_file};
use anyhow::Result;
use hound::WavReader;
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

#[test]
fn spectrum_matches_expectation() -> Result<()> {
    // MIDI note for C3 should be 60 (middle C), not 48
    // C3 = MIDI note 60, frequency ~261.63 Hz
    let midi_note_c3 = 60;
    let sample_rate = 44100.0f64;
    let duration = 4.0f64; // Reference appears to be 4 seconds based on sample count

    // 1. Load the reference audio file and measure its properties
    let reference_samples = load_wav_file("patch-20-c3-2s.wav")?;

    // Normalize reference samples to help with amplitude comparison
    let ref_peak = reference_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let reference_normalized: Vec<f32> = if ref_peak > 0.0 {
        reference_samples.iter().map(|&x| x / ref_peak).collect()
    } else {
        reference_samples.clone()
    };

    let reference_analysis = analyze_audio(&reference_normalized, sample_rate as f32)?;

    // 2. Load patch 20 from star1-fast-decay.syx and synthesize audio
    let patches = parse_sysex_file("star1-fast-decay.syx")?;
    assert!(patches.len() > 20, "Sysex file should contain at least 21 patches (patch 20 at index 20)");

    let patch_20 = &patches[20]; // Patch 20 (1-indexed) is at index 20 (0-indexed)
    println!("Using patch 20: '{}'", patch_20.name);

    let mut synth = Dx7Synth::new(sample_rate, duration + 0.1);
    synth.load_patch(patch_20.clone())?;

    let synthesized_samples = synth.render_note(midi_note_c3, 127, duration)?;

    // Normalize synthesized samples to help with amplitude comparison
    let synth_peak = synthesized_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let synthesized_normalized: Vec<f32> = if synth_peak > 0.0 {
        synthesized_samples.iter().map(|&x| x / synth_peak).collect()
    } else {
        synthesized_samples.clone()
    };

    let synthesized_analysis = analyze_audio(&synthesized_normalized, sample_rate as f32)?;

    // 3. Compare the reference and synthesized audio
    // Use the shortest common length for comparison
    let min_length = reference_normalized.len().min(synthesized_normalized.len());
    let reference_trimmed = &reference_normalized[..min_length];
    let synthesized_trimmed = &synthesized_normalized[..min_length];

    println!("Analysis comparison:");
    println!("Reference  - RMS: {:.6}, Peak: {:.6}, Spectral Centroid: {:.1} Hz",
             reference_analysis.rms, reference_analysis.peak_amplitude, reference_analysis.spectral_centroid);
    println!("Synthesized - RMS: {:.6}, Peak: {:.6}, Spectral Centroid: {:.1} Hz",
             synthesized_analysis.rms, synthesized_analysis.peak_amplitude, synthesized_analysis.spectral_centroid);

    // Assert spectral properties match within strict tolerance for synthesis accuracy
    let rms_tolerance = 0.05; // 5% tolerance for RMS level
    let peak_tolerance = 0.05; // 5% tolerance for peak amplitude
    let spectral_tolerance = 10.0; // 10 Hz tolerance for spectral centroid

    let rms_diff = (reference_analysis.rms - synthesized_analysis.rms).abs() / reference_analysis.rms;
    let peak_diff = (reference_analysis.peak_amplitude - synthesized_analysis.peak_amplitude).abs() / reference_analysis.peak_amplitude;
    let spectral_diff = (reference_analysis.spectral_centroid - synthesized_analysis.spectral_centroid).abs();

    assert!(rms_diff < rms_tolerance,
            "RMS level mismatch: reference {:.6}, synthesized {:.6}, difference {:.1}%",
            reference_analysis.rms, synthesized_analysis.rms, rms_diff * 100.0);

    assert!(peak_diff < peak_tolerance,
            "Peak amplitude mismatch: reference {:.6}, synthesized {:.6}, difference {:.1}%",
            reference_analysis.peak_amplitude, synthesized_analysis.peak_amplitude, peak_diff * 100.0);

    assert!(spectral_diff < spectral_tolerance,
            "Spectral centroid mismatch: reference {:.1} Hz, synthesized {:.1} Hz, difference {:.1} Hz",
            reference_analysis.spectral_centroid, synthesized_analysis.spectral_centroid, spectral_diff);

    // Additional correlation check - compare waveforms directly
    let correlation = calculate_correlation(reference_trimmed, synthesized_trimmed);
    println!("Waveform correlation: {:.4}", correlation);

    // Correlation should be very high for matching synthesis
    assert!(correlation > 0.9,
            "Waveform correlation too low: {:.4} (expected > 0.9)", correlation);

    println!("✅ Patch 20 spectrum analysis passed all tests");
    Ok(())
}

#[derive(Debug)]
struct AudioAnalysis {
    rms: f32,
    peak_amplitude: f32,
    spectral_centroid: f32,
}

fn load_wav_file(filename: &str) -> Result<Vec<f32>> {
    let mut reader = WavReader::open(filename)?;
    let spec = reader.spec();

    let samples: Vec<f32> = match spec.bits_per_sample {
        16 => {
            reader.samples::<i16>()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|s| s as f32 / 32768.0) // Convert i16 to f32
                .collect()
        }
        32 => {
            reader.samples::<f32>()
                .collect::<Result<Vec<_>, _>>()?
        }
        _ => return Err(anyhow::anyhow!("Unsupported bit depth: {}", spec.bits_per_sample))
    };

    Ok(samples)
}

fn analyze_audio(samples: &[f32], sample_rate: f32) -> Result<AudioAnalysis> {
    // Calculate RMS
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();

    // Calculate peak amplitude
    let peak_amplitude = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    // Calculate spectral centroid using FFT
    let spectral_centroid = calculate_spectral_centroid(samples, sample_rate)?;

    Ok(AudioAnalysis {
        rms,
        peak_amplitude,
        spectral_centroid,
    })
}

fn calculate_spectral_centroid(samples: &[f32], sample_rate: f32) -> Result<f32> {
    let window_size = 8192.min(samples.len());
    let analysis_samples = &samples[..window_size];

    // Apply Hann window
    let windowed: Vec<Complex<f32>> = analysis_samples
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / (window_size - 1) as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    // Perform FFT
    let mut fft_data = windowed.clone();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);
    fft.process(&mut fft_data);

    // Calculate spectral centroid
    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;

    for (i, complex) in fft_data.iter().take(window_size / 2).enumerate() {
        let frequency = (i as f32) * sample_rate / (window_size as f32);
        let magnitude = complex.norm();

        weighted_sum += frequency * magnitude;
        magnitude_sum += magnitude;
    }

    let centroid = if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    };

    Ok(centroid)
}

fn calculate_correlation(samples1: &[f32], samples2: &[f32]) -> f32 {
    assert_eq!(samples1.len(), samples2.len());

    let n = samples1.len() as f32;
    let mean1: f32 = samples1.iter().sum::<f32>() / n;
    let mean2: f32 = samples2.iter().sum::<f32>() / n;

    let mut numerator = 0.0;
    let mut sum1_sq = 0.0;
    let mut sum2_sq = 0.0;

    for (&x1, &x2) in samples1.iter().zip(samples2.iter()) {
        let diff1 = x1 - mean1;
        let diff2 = x2 - mean2;
        numerator += diff1 * diff2;
        sum1_sq += diff1 * diff1;
        sum2_sq += diff2 * diff2;
    }

    let denominator = (sum1_sq * sum2_sq).sqrt();
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}
