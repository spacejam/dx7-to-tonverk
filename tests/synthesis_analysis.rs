use dx7tv::{Dx7Synth, sysex::parse_sysex_file};
use anyhow::Result;
use hound::WavReader;
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

/// Comprehensive analysis of synthesis differences
#[test]
fn analyze_synthesis_differences() -> Result<()> {
    // Load reference and synthesized audio
    let reference_samples = load_wav_file("patch-20-c3-2s.wav")?;

    // Generate synthesized audio
    let patches = parse_sysex_file("star1-fast-decay.syx")?;
    let patch_20 = &patches[20];

    let mut synth = Dx7Synth::new(44100.0, 4.0);
    synth.load_patch(patch_20.clone())?;
    let synthesized_samples = synth.render_note(60, 127, 4.0)?;

    println!("=== SYNTHESIS ANALYSIS ===");
    println!("Reference samples: {}", reference_samples.len());
    println!("Synthesized samples: {}", synthesized_samples.len());

    // Time-domain analysis
    analyze_time_domain(&reference_samples, &synthesized_samples)?;

    // Frequency-domain analysis
    analyze_frequency_domain(&reference_samples, &synthesized_samples)?;

    // Envelope analysis
    analyze_envelope_behavior(&reference_samples, &synthesized_samples)?;

    Ok(())
}

fn analyze_time_domain(reference: &[f32], synthesized: &[f32]) -> Result<()> {
    println!("\n=== TIME DOMAIN ANALYSIS ===");

    let min_len = reference.len().min(synthesized.len());
    let ref_slice = &reference[..min_len];
    let synth_slice = &synthesized[..min_len];

    // Basic statistics
    let ref_rms = (ref_slice.iter().map(|&x| x * x).sum::<f32>() / min_len as f32).sqrt();
    let synth_rms = (synth_slice.iter().map(|&x| x * x).sum::<f32>() / min_len as f32).sqrt();

    let ref_peak = ref_slice.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let synth_peak = synth_slice.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("RMS - Reference: {:.6}, Synthesized: {:.6}, Ratio: {:.3}",
             ref_rms, synth_rms, synth_rms / ref_rms);
    println!("Peak - Reference: {:.6}, Synthesized: {:.6}, Ratio: {:.3}",
             ref_peak, synth_peak, synth_peak / ref_peak);

    // Analyze first few seconds in detail
    let one_sec = 44100;
    if min_len >= one_sec {
        for sec in 0..4 {
            let start = sec * one_sec;
            let end = (start + one_sec).min(min_len);
            let ref_sec = &ref_slice[start..end];
            let synth_sec = &synth_slice[start..end];

            let ref_rms_sec = (ref_sec.iter().map(|&x| x * x).sum::<f32>() / (end - start) as f32).sqrt();
            let synth_rms_sec = (synth_sec.iter().map(|&x| x * x).sum::<f32>() / (end - start) as f32).sqrt();

            println!("Second {} - Ref RMS: {:.6}, Synth RMS: {:.6}, Ratio: {:.3}",
                     sec, ref_rms_sec, synth_rms_sec, synth_rms_sec / ref_rms_sec);
        }
    }

    Ok(())
}

fn analyze_frequency_domain(reference: &[f32], synthesized: &[f32]) -> Result<()> {
    println!("\n=== FREQUENCY DOMAIN ANALYSIS ===");

    // Analyze first 8192 samples of each
    let window_size = 8192;
    let ref_window = &reference[..window_size.min(reference.len())];
    let synth_window = &synthesized[..window_size.min(synthesized.len())];

    let ref_spectrum = compute_spectrum(ref_window)?;
    let synth_spectrum = compute_spectrum(synth_window)?;

    // Find fundamental and harmonics
    let fundamental_freq = 261.63; // C3
    println!("Expected fundamental: {:.2} Hz", fundamental_freq);

    // Find peaks in both spectra
    find_spectral_peaks(&ref_spectrum, "Reference")?;
    find_spectral_peaks(&synth_spectrum, "Synthesized")?;

    // Compare spectral centroids
    let ref_centroid = calculate_spectral_centroid(ref_window, 44100.0)?;
    let synth_centroid = calculate_spectral_centroid(synth_window, 44100.0)?;

    println!("Spectral Centroid - Reference: {:.1} Hz, Synthesized: {:.1} Hz, Diff: {:.1} Hz",
             ref_centroid, synth_centroid, (synth_centroid - ref_centroid).abs());

    Ok(())
}

fn analyze_envelope_behavior(reference: &[f32], synthesized: &[f32]) -> Result<()> {
    println!("\n=== ENVELOPE ANALYSIS ===");

    let window_size = 1024; // ~23ms windows
    let min_len = reference.len().min(synthesized.len());

    println!("Window analysis (RMS over time):");
    for (i, chunk_start) in (0..min_len).step_by(window_size).take(20).enumerate() {
        let chunk_end = (chunk_start + window_size).min(min_len);
        let ref_chunk = &reference[chunk_start..chunk_end];
        let synth_chunk = &synthesized[chunk_start..chunk_end];

        let ref_rms = (ref_chunk.iter().map(|&x| x * x).sum::<f32>() / (chunk_end - chunk_start) as f32).sqrt();
        let synth_rms = (synth_chunk.iter().map(|&x| x * x).sum::<f32>() / (chunk_end - chunk_start) as f32).sqrt();

        let time_ms = (chunk_start as f32 / 44100.0) * 1000.0;
        println!("  {:.0}ms - Ref: {:.6}, Synth: {:.6}, Ratio: {:.3}",
                 time_ms, ref_rms, synth_rms, if ref_rms > 0.0 { synth_rms / ref_rms } else { 0.0 });
    }

    Ok(())
}

fn compute_spectrum(samples: &[f32]) -> Result<Vec<f32>> {
    let window_size = samples.len();

    // Apply Hann window
    let windowed: Vec<Complex<f32>> = samples
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

    // Convert to magnitude spectrum
    let spectrum: Vec<f32> = fft_data.iter()
        .take(window_size / 2)
        .map(|c| c.norm())
        .collect();

    Ok(spectrum)
}

fn find_spectral_peaks(spectrum: &[f32], label: &str) -> Result<()> {
    let sample_rate = 44100.0;
    let window_size = spectrum.len() * 2;

    // Find top 5 peaks
    let mut peaks: Vec<(usize, f32)> = spectrum.iter()
        .enumerate()
        .map(|(i, &mag)| (i, mag))
        .collect();

    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("{} - Top 5 spectral peaks:", label);
    for (i, (bin, magnitude)) in peaks.iter().take(5).enumerate() {
        let frequency = (*bin as f32) * sample_rate / (window_size as f32);
        println!("  {}: {:.1} Hz (mag: {:.6})", i + 1, frequency, magnitude);
    }

    Ok(())
}

fn calculate_spectral_centroid(samples: &[f32], sample_rate: f32) -> Result<f32> {
    let spectrum = compute_spectrum(samples)?;

    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;

    for (i, &magnitude) in spectrum.iter().enumerate() {
        let frequency = (i as f32) * sample_rate / (samples.len() as f32);
        weighted_sum += frequency * magnitude;
        magnitude_sum += magnitude;
    }

    Ok(if magnitude_sum > 0.0 { weighted_sum / magnitude_sum } else { 0.0 })
}

fn load_wav_file(filename: &str) -> Result<Vec<f32>> {
    let mut reader = WavReader::open(filename)?;
    let spec = reader.spec();

    let samples: Vec<f32> = match spec.bits_per_sample {
        16 => {
            reader.samples::<i16>()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|s| s as f32 / 32768.0)
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