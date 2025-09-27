use anyhow::Result;
use dx7tv::{Dx7Synth, parse_sysex_file, synth::midi_note_to_frequency};
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

/// Test that MIDI note pitch is correctly applied to operators
///
/// This test verifies that different MIDI notes produce different fundamental frequencies
/// in the synthesized output, using FFT analysis to measure the spectral content.
#[test]
fn test_midi_pitch_application() -> Result<()> {
    let sample_rate = 44100.0;
    let note_length = 0.5; // 500ms should be enough for analysis

    // Create a simple patch with only one carrier operator producing a sine wave
    let mut synth = Dx7Synth::new(sample_rate, note_length + 0.1);

    // Create a minimal test patch - single operator producing a sine wave
    let mut patch = dx7tv::sysex::Dx7Patch::new("SINE TEST");

    // Set algorithm 1 (stored as 0) - single carrier
    patch.global.algorithm = 0;

    // Configure operator 5 (which maps to sysex operator 0 - carrier in algorithm 1)
    patch.operators[5].rates.attack = 99;     // Maximum attack rate for instant sound
    patch.operators[5].rates.decay1 = 99;    // Fast decay1
    patch.operators[5].rates.decay2 = 99;    // Fast decay2
    patch.operators[5].rates.release = 50;   // Medium release

    patch.operators[5].levels.attack = 99;   // Maximum attack level
    patch.operators[5].levels.decay1 = 99;   // Full level
    patch.operators[5].levels.decay2 = 99;   // Full level
    patch.operators[5].levels.release = 0;   // Silent release

    patch.operators[5].output_level = 99;    // Maximum output
    patch.operators[5].coarse_freq = 1;      // 1:1 frequency ratio
    patch.operators[5].fine_freq = 0;        // No fine tuning
    patch.operators[5].detune = 7;           // Center detune
    patch.operators[5].osc_mode = 0;         // Ratio mode (follows MIDI note)

    // Ensure all other operators are silent
    for i in 0..5 {
        patch.operators[i].output_level = 0;
    }

    synth.load_patch(patch)?;

    // Test MIDI note 60 (C4, ~261.63 Hz)
    let samples_60 = synth.render_note(60, 127, note_length)?;
    let freq_60 = find_fundamental_frequency(&samples_60, sample_rate as f32)?;
    let expected_freq_60 = midi_note_to_frequency(60) as f32;

    println!("MIDI 60: Expected {:.2} Hz, Measured {:.2} Hz", expected_freq_60, freq_60);

    // Reset synth for next note
    synth.reset();

    // Test MIDI note 72 (C5, ~523.25 Hz - one octave higher)
    let samples_72 = synth.render_note(72, 127, note_length)?;
    let freq_72 = find_fundamental_frequency(&samples_72, sample_rate as f32)?;
    let expected_freq_72 = midi_note_to_frequency(72) as f32;

    println!("MIDI 72: Expected {:.2} Hz, Measured {:.2} Hz", expected_freq_72, freq_72);

    // Verify the frequencies are approximately correct (within 5%)
    let tolerance = 0.05; // 5% tolerance for FFT analysis

    assert!(
        (freq_60 - expected_freq_60).abs() / expected_freq_60 < tolerance,
        "MIDI note 60 frequency incorrect: expected {:.2} Hz, got {:.2} Hz (error: {:.1}%)",
        expected_freq_60, freq_60, (freq_60 - expected_freq_60).abs() / expected_freq_60 * 100.0
    );

    assert!(
        (freq_72 - expected_freq_72).abs() / expected_freq_72 < tolerance,
        "MIDI note 72 frequency incorrect: expected {:.2} Hz, got {:.2} Hz (error: {:.1}%)",
        expected_freq_72, freq_72, (freq_72 - expected_freq_72).abs() / expected_freq_72 * 100.0
    );

    // Verify the octave relationship (72 should be ~2x the frequency of 60)
    let octave_ratio = freq_72 / freq_60;
    assert!(
        (octave_ratio - 2.0).abs() < 0.1,
        "Octave relationship incorrect: MIDI 72 should be 2x MIDI 60, got ratio {:.3}",
        octave_ratio
    );

    Ok(())
}

/// Find the fundamental frequency using FFT analysis
///
/// Performs FFT on the audio samples and finds the peak frequency
fn find_fundamental_frequency(samples: &[f32], sample_rate: f32) -> Result<f32> {
    if samples.is_empty() {
        return Err(anyhow::anyhow!("No samples provided"));
    }

    // Use a window of samples for analysis (e.g., first 8192 samples)
    let window_size = 8192.min(samples.len());
    let analysis_samples = &samples[..window_size];

    // Apply Hann window to reduce spectral leakage
    let mut windowed_samples: Vec<Complex<f32>> = analysis_samples
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / (window_size - 1) as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    // Perform FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);
    fft.process(&mut windowed_samples);

    // Find the peak frequency (ignore DC and very low frequencies)
    let min_bin = (50.0 * window_size as f32 / sample_rate).ceil() as usize; // Ignore below 50 Hz
    let max_bin = window_size / 2; // Nyquist limit

    let mut max_magnitude = 0.0;
    let mut peak_bin = min_bin;

    for i in min_bin..max_bin {
        let magnitude = windowed_samples[i].norm();
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            peak_bin = i;
        }
    }

    // Convert bin to frequency
    let fundamental_freq = (peak_bin as f32) * sample_rate / (window_size as f32);

    // Verify we found a reasonable signal
    if max_magnitude < 0.01 {
        return Err(anyhow::anyhow!("Signal too weak for frequency analysis"));
    }

    Ok(fundamental_freq)
}

/// Test with actual DX7 sysex file if available
#[test]
fn test_sysex_midi_pitch_application() -> Result<()> {
    // Try to find a sysex file for testing
    let sysex_paths = [
        "star1-fast-decay.syx",
        "test.syx",
        "../star1-fast-decay.syx",
        "../test.syx"
    ];

    let mut sysex_file = None;
    for path in &sysex_paths {
        if std::path::Path::new(path).exists() {
            sysex_file = Some(*path);
            break;
        }
    }

    let Some(sysex_path) = sysex_file else {
        println!("Skipping sysex test - no sysex file found");
        return Ok(());
    };

    let patches = parse_sysex_file(sysex_path)?;
    if patches.is_empty() {
        return Err(anyhow::anyhow!("No patches found in sysex file"));
    }

    let sample_rate = 44100.0;
    let note_length = 0.5;

    let mut synth = Dx7Synth::new(sample_rate, note_length + 0.1);
    synth.load_patch(patches[0].clone())?;

    // Test two different MIDI notes
    let samples_60 = synth.render_note(60, 100, note_length)?;
    synth.reset();
    let samples_69 = synth.render_note(69, 100, note_length)?; // A4 = 440 Hz

    // Check that the samples are different (indicating pitch change)
    let are_different = samples_60.iter()
        .zip(samples_69.iter())
        .take(1000) // Check first 1000 samples
        .any(|(&a, &b)| (a - b).abs() > 0.01);

    assert!(
        are_different,
        "Samples for MIDI notes 60 and 69 are identical - pitch not being applied"
    );

    // Try FFT analysis if samples have enough signal
    if let (Ok(freq_60), Ok(freq_69)) = (
        find_fundamental_frequency(&samples_60, sample_rate as f32),
        find_fundamental_frequency(&samples_69, sample_rate as f32)
    ) {
        println!("Sysex test - MIDI 60: {:.2} Hz, MIDI 69: {:.2} Hz", freq_60, freq_69);

        // Note 69 should be higher than note 60
        assert!(
            freq_69 > freq_60,
            "MIDI note 69 should have higher frequency than MIDI note 60: {:.2} Hz vs {:.2} Hz",
            freq_69, freq_60
        );
    }

    Ok(())
}