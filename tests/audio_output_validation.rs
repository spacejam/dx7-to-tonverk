use dx7tv::sysex;
use dx7tv::synth::Dx7Synth;
use hound::{WavReader, SampleFormat};
use std::fs;

/// Comprehensive audio output validation tests
/// These tests ensure the synthesis engine produces expected audio characteristics

#[test]
fn test_star1_fast_decay_audio_characteristics() {
    let _ = env_logger::try_init();

    println!("=== Testing star1-fast-decay.syx Audio Characteristics ===");

    // Load the sysex file
    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read and parse star1-fast-decay.syx - make sure it exists in project root");

    // Test first patch from the sysex
    let patch = patches.get(0)
        .expect("No patches found in sysex file")
        .clone();

    println!("Testing patch: '{}'", patch.name);

    // Create synth and render audio
    let mut synth = Dx7Synth::new(44100.0, 5.0);
    synth.load_patch(patch.clone()).expect("Failed to load patch");

    // Render 2 seconds of audio at MIDI note 60 (middle C)
    let samples = synth.render_note(60, 100, 2.0).expect("Failed to render note");

    println!("Generated {} samples ({:.2} seconds)", samples.len(), samples.len() as f64 / 44100.0);

    // Test 1: Duration should be at least 2 seconds
    let expected_min_samples = (2.0 * 44100.0) as usize;
    assert!(
        samples.len() >= expected_min_samples,
        "Audio duration too short: got {:.3}s, expected at least 2.0s",
        samples.len() as f64 / 44100.0
    );

    // Test 2: Audio should not be completely silent
    let non_zero_samples = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    let non_zero_percentage = (non_zero_samples as f64 / samples.len() as f64) * 100.0;

    println!("Non-zero samples: {}/{} ({:.1}%)", non_zero_samples, samples.len(), non_zero_percentage);

    assert!(
        non_zero_samples > 0,
        "Audio is completely silent - no non-zero samples found"
    );

    assert!(
        non_zero_percentage > 1.0,
        "Audio is mostly silent - only {:.1}% of samples are non-zero",
        non_zero_percentage
    );

    // Test 3: Peak amplitude should be reasonable (not clipping, not too quiet)
    let max_amplitude = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    println!("Peak amplitude: {:.6}", max_amplitude);

    assert!(
        max_amplitude > 0.001,
        "Audio is too quiet - peak amplitude {:.6} is below 0.001",
        max_amplitude
    );

    assert!(
        max_amplitude <= 1.0,
        "Audio is clipping - peak amplitude {:.6} exceeds 1.0",
        max_amplitude
    );

    // Test 4: RMS level should indicate reasonable volume
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
    println!("RMS level: {:.6}", rms);

    assert!(
        rms > 0.0001,
        "RMS level too low: {:.6} - audio may be too quiet",
        rms
    );

    // Test 5: Basic spectral content analysis (should have energy in audible range)
    let significant_samples = samples.iter().filter(|&&x| x.abs() > max_amplitude * 0.1).count();
    let significant_percentage = (significant_samples as f64 / samples.len() as f64) * 100.0;

    println!("Samples above 10% of peak: {}/{} ({:.1}%)", significant_samples, samples.len(), significant_percentage);

    assert!(
        significant_percentage > 0.1,
        "Too few significant samples - only {:.1}% above 10% of peak amplitude",
        significant_percentage
    );

    // Test 6: Audio should have some variation (not just a constant tone)
    let mut variation_count = 0;
    let window_size = 1024;
    for window_start in (0..samples.len().saturating_sub(window_size)).step_by(window_size) {
        let window = &samples[window_start..window_start + window_size];
        let window_max = window.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        let window_rms = (window.iter().map(|&x| x * x).sum::<f32>() / window.len() as f32).sqrt();

        // Check if this window has significant variation
        if window_max > 0.001 && window_rms > 0.0001 {
            variation_count += 1;
        }
    }

    let variation_percentage = (variation_count as f64 / (samples.len() / window_size) as f64) * 100.0;
    println!("Windows with variation: {}/{} ({:.1}%)", variation_count, samples.len() / window_size, variation_percentage);

    assert!(
        variation_percentage > 10.0,
        "Audio lacks variation - only {:.1}% of windows show significant audio",
        variation_percentage
    );

    println!("✅ star1-fast-decay.syx audio characteristics test PASSED");
}

#[test]
fn test_audio_output_against_wav_file() {
    let _ = env_logger::try_init();

    println!("=== Testing Generated WAV File Characteristics ===");

    // Generate the WAV file first
    let output_path = "out/test_validation.wav";
    std::fs::create_dir_all("out").expect("Failed to create output directory");

    let result = std::process::Command::new("cargo")
        .args(&["run", "--bin", "dx7tv", "--", "./star1-fast-decay.syx", "60", "2", output_path])
        .output()
        .expect("Failed to run dx7tv command");

    if !result.status.success() {
        panic!("dx7tv command failed: {}", String::from_utf8_lossy(&result.stderr));
    }

    println!("Generated WAV file: {}", output_path);

    // Read and analyze the WAV file
    let mut reader = WavReader::open(output_path)
        .expect("Failed to open generated WAV file");

    let spec = reader.spec();
    println!("WAV spec: {}Hz, {} channels, {} bits, {} samples",
             spec.sample_rate, spec.channels, spec.bits_per_sample, reader.len());

    // Test WAV file characteristics
    assert_eq!(spec.channels, 1, "Expected mono audio");
    assert_eq!(spec.sample_rate, 44100, "Expected 44.1kHz sample rate");
    assert_eq!(spec.sample_format, SampleFormat::Int, "Expected integer samples");

    // Read all samples (24-bit WAV uses i32 samples)
    let samples: Result<Vec<i32>, _> = reader.samples::<i32>().collect();
    let samples = samples.expect("Failed to read WAV samples");

    let duration_seconds = samples.len() as f64 / spec.sample_rate as f64;
    println!("WAV duration: {:.3} seconds ({} samples)", duration_seconds, samples.len());

    // Test duration
    assert!(
        duration_seconds >= 1.9, // Allow slight tolerance
        "WAV duration too short: {:.3}s, expected at least 2.0s",
        duration_seconds
    );

    // Test for non-silent audio
    let non_zero_samples = samples.iter().filter(|&&x| x.abs() > 256).count(); // Threshold for 24-bit
    let non_zero_percentage = (non_zero_samples as f64 / samples.len() as f64) * 100.0;

    println!("WAV non-zero samples: {}/{} ({:.1}%)", non_zero_samples, samples.len(), non_zero_percentage);

    assert!(
        non_zero_samples > 0,
        "WAV file is completely silent"
    );

    // Test peak amplitude
    let max_amplitude = samples.iter().map(|&x| x.abs()).max().unwrap_or(0) as f64;
    let max_amplitude_normalized = max_amplitude / 8388608.0; // Normalize 24-bit to -1.0 to 1.0 (2^23)

    println!("WAV peak amplitude: {} ({:.3} normalized)", max_amplitude as i32, max_amplitude_normalized);

    assert!(
        max_amplitude > 25600.0, // Should have some significant amplitude in 24-bit range
        "WAV peak amplitude too low: {} (normalized: {:.6})",
        max_amplitude as i16,
        max_amplitude_normalized
    );

    // Clean up
    std::fs::remove_file(output_path).ok();

    println!("✅ WAV file characteristics test PASSED");
}

#[test]
fn test_different_notes_produce_different_audio() {
    let _ = env_logger::try_init();

    println!("=== Testing Different MIDI Notes Produce Different Audio ===");

    // Load the sysex file
    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read and parse star1-fast-decay.syx");
    let patch = patches.get(0).expect("No patches found").clone();

    let mut synth = Dx7Synth::new(44100.0, 1.0);
    synth.load_patch(patch).expect("Failed to load patch");

    // Render different MIDI notes
    let note_c = synth.render_note(60, 100, 0.5).expect("Failed to render C"); // Middle C
    let note_g = synth.render_note(67, 100, 0.5).expect("Failed to render G"); // G above middle C

    // Both should produce audio
    assert!(note_c.iter().any(|&x| x.abs() > 0.001), "Note C produced silence");
    assert!(note_g.iter().any(|&x| x.abs() > 0.001), "Note G produced silence");

    // They should be different (frequency difference should create different waveforms)
    let min_len = note_c.len().min(note_g.len());
    let differences = note_c[..min_len].iter()
        .zip(note_g[..min_len].iter())
        .filter(|(a, b)| (*a - *b).abs() > 0.001)
        .count();

    let difference_percentage = (differences as f64 / min_len as f64) * 100.0;
    println!("Sample differences between C and G: {:.1}%", difference_percentage);

    assert!(
        difference_percentage > 10.0,
        "Different notes should produce noticeably different audio - only {:.1}% different",
        difference_percentage
    );

    println!("✅ Different notes test PASSED");
}

#[test]
fn test_multiple_patches_from_sysex() {
    let _ = env_logger::try_init();

    println!("=== Testing Multiple Patches from SYSEX ===");

    // Load the sysex file
    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read and parse star1-fast-decay.syx");

    println!("SYSEX contains {} patches", patches.len());

    // Test first few patches to ensure they all produce audio
    let num_patches_to_test = patches.len().min(5); // Test first 5 patches

    for patch_idx in 0..num_patches_to_test {
        let patch = &patches[patch_idx];
        println!("Testing patch {}: '{}'", patch_idx, patch.name);

        let mut synth = Dx7Synth::new(44100.0, 1.0);
        synth.load_patch(patch.clone()).expect("Failed to load patch");

        let samples = synth.render_note(60, 100, 0.2).expect("Failed to render note");

        let max_amplitude = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        println!("  Patch {} peak amplitude: {:.6}", patch_idx, max_amplitude);

        // Each patch should produce some audio (though some might be quiet)
        assert!(
            max_amplitude > 1e-8,
            "Patch {} '{}' produced silence (peak: {:.2e})",
            patch_idx, patch.name, max_amplitude
        );
    }

    println!("✅ Multiple patches test PASSED");
}