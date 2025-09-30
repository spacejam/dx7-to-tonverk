use dx7tv::{render_patch, parse_sysex_file, Dx7Patch};
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

/// Perform FFT analysis on audio samples to find dominant frequency
fn analyze_frequency(samples: &[f32], sample_rate: f32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    // Use a power of 2 size for efficient FFT
    let fft_size = 1024.min(samples.len());
    let mut fft_input: Vec<Complex<f32>> = samples[..fft_size]
        .iter()
        .map(|&s| Complex::new(s, 0.0))
        .collect();

    // Apply Hamming window to reduce spectral leakage
    for (i, sample) in fft_input.iter_mut().enumerate() {
        let window = 0.54 - 0.46 * (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos();
        *sample *= window;
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    fft.process(&mut fft_input);

    // Find the peak frequency (excluding DC component at index 0)
    let mut max_magnitude = 0.0;
    let mut peak_bin = 0;

    for (i, &complex) in fft_input.iter().enumerate().skip(1).take(fft_size / 2) {
        let magnitude = complex.norm();
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            peak_bin = i;
        }
    }

    // Convert bin to frequency
    (peak_bin as f32 * sample_rate) / fft_size as f32
}

/// Test MIDI note to frequency conversion using actual synthesis
#[test]
fn test_midi_note_60_frequency() {
    env_logger::try_init().ok();

    // Create a simple sine wave patch
    let mut patch = Dx7Patch::new("SINE TEST");

    // Configure operator 0 as a pure sine wave carrier
    patch.operators[0].output_level = 99;
    patch.operators[0].rates.attack = 99;     // Fast attack
    patch.operators[0].rates.decay1 = 99;     // No decay
    patch.operators[0].rates.decay2 = 99;     // No decay
    patch.operators[0].rates.release = 0;     // No release during sustain
    patch.operators[0].levels.attack = 99;    // Full level
    patch.operators[0].levels.decay1 = 99;    // Full level
    patch.operators[0].levels.decay2 = 99;    // Full level
    patch.operators[0].levels.release = 0;    // Silent on release
    patch.operators[0].coarse_freq = 1;       // 1:1 frequency ratio
    patch.operators[0].fine_freq = 0;         // No fine tuning
    patch.operators[0].detune = 7;            // Center detune
    patch.global.algorithm = 0;               // Algorithm 1: OP0 as carrier

    // Render MIDI note 60 (C4, ~261.63 Hz)
    let samples = render_patch(patch, 60, 0.5).expect("Failed to render patch");

    // Analyze frequency content
    let detected_freq = analyze_frequency(&samples, 48000.0);
    let expected_freq = 261.63; // C4 frequency

    log::info!("MIDI note 60: detected frequency = {:.2} Hz, expected = {:.2} Hz", detected_freq, expected_freq);

    // Allow 2% tolerance for frequency accuracy
    let tolerance = expected_freq * 0.02;
    assert!((detected_freq - expected_freq).abs() < tolerance,
        "Frequency mismatch: detected {:.2} Hz, expected {:.2} Hz (±{:.2} Hz)",
        detected_freq, expected_freq, tolerance);

    // Ensure we got meaningful audio output (not silence)
    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!(rms > 0.001, "Audio output too quiet, RMS = {}", rms);
}

#[test]
fn test_multiple_midi_notes() {
    env_logger::try_init().ok();

    // Create a simple sine wave patch
    let mut patch = Dx7Patch::new("SINE TEST");

    // Configure operator 0 as a pure sine wave carrier
    patch.operators[0].output_level = 99;
    patch.operators[0].rates.attack = 99;
    patch.operators[0].rates.decay1 = 99;
    patch.operators[0].rates.decay2 = 99;
    patch.operators[0].rates.release = 0;
    patch.operators[0].levels.attack = 99;
    patch.operators[0].levels.decay1 = 99;
    patch.operators[0].levels.decay2 = 99;
    patch.operators[0].levels.release = 0;
    patch.operators[0].coarse_freq = 1;
    patch.operators[0].fine_freq = 0;
    patch.operators[0].detune = 7;
    patch.global.algorithm = 0;

    let test_cases = vec![
        (48, 130.81), // C3
        (60, 261.63), // C4
        (72, 523.25), // C5
        (69, 440.00), // A4
    ];

    for (midi_note, expected_freq) in test_cases {
        let samples = render_patch(patch.clone(), midi_note, 0.5).expect("Failed to render patch");
        let detected_freq = analyze_frequency(&samples, 48000.0);

        log::info!("MIDI note {}: detected frequency = {:.2} Hz, expected = {:.2} Hz",
                  midi_note, detected_freq, expected_freq);

        // Allow 3% tolerance for frequency accuracy
        let tolerance = expected_freq * 0.03;
        assert!((detected_freq - expected_freq).abs() < tolerance,
            "MIDI note {} frequency mismatch: detected {:.2} Hz, expected {:.2} Hz (±{:.2} Hz)",
            midi_note, detected_freq, expected_freq, tolerance);

        // Ensure we got meaningful audio output
        let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        assert!(rms > 0.001, "MIDI note {} audio output too quiet, RMS = {}", midi_note, rms);
    }
}

#[test]
fn test_frequency_with_real_patch() {
    env_logger::try_init().ok();

    // Try to load a real patch file if available
    if let Ok(patches) = parse_sysex_file("star1-fast-decay.syx") {
        let patch = &patches[0];

        // Render MIDI note 60
        let samples = render_patch(patch.clone(), 60, 0.3).expect("Failed to render real patch");

        // For real patches, we just verify the audio isn't silent and has reasonable frequency content
        let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        log::info!("Real patch '{}' RMS level: {}", patch.name, rms);

        // Real patches might be quieter than our test sine wave, so lower threshold
        assert!(rms > 0.0001, "Real patch audio output too quiet, RMS = {}", rms);

        // Analyze frequency content
        let detected_freq = analyze_frequency(&samples, 48000.0);
        log::info!("Real patch '{}' dominant frequency: {:.2} Hz", patch.name, detected_freq);

        // For complex patches, just verify we detect some reasonable frequency
        assert!(detected_freq > 50.0 && detected_freq < 2000.0,
            "Real patch detected frequency {:.2} Hz seems unreasonable", detected_freq);
    }
}