//! Test individual synthesis components against expected values
//! This will help identify exactly where the audio path breaks

use dx7tv::synth::Dx7Synth;
use std::f64::consts::PI;

#[test]
fn test_basic_frequency_calculation() {
    println!("=== Testing Basic Frequency Calculation ===");

    // Test MIDI note to frequency conversion
    let midi_note = 60;
    let expected_base_freq = 440.0 * 2.0_f64.powf((60.0 - 69.0) / 12.0);

    println!("MIDI note {}: expected base freq = {:.2} Hz", midi_note, expected_base_freq);
    assert!((expected_base_freq - 261.63).abs() < 0.1, "Base frequency calculation is wrong");

    // Test DX7 coarse multipliers
    let coarse_0_freq = expected_base_freq * 0.5;  // Coarse 0 = 0.5x ratio
    let coarse_1_freq = expected_base_freq * 1.0;  // Coarse 1 = 1.0x ratio

    println!("Coarse 0 (0.5x): {:.2} Hz", coarse_0_freq);
    println!("Coarse 1 (1.0x): {:.2} Hz", coarse_1_freq);

    assert!((coarse_0_freq - 130.81).abs() < 1.0, "Coarse 0 frequency wrong");
    assert!((coarse_1_freq - 261.63).abs() < 1.0, "Coarse 1 frequency wrong");
}

#[test]
fn test_synth_creation() {
    println!("=== Testing Synth Creation ===");

    let synth = Dx7Synth::new(44100.0, 10.0);
    println!("Created synth with sample rate: {}", synth.sample_rate());

    assert_eq!(synth.sample_rate(), 44100.0);
    assert_eq!(synth.active_voices(), 0);

    println!("Synth creation: PASS");
}

#[test]
fn test_patch_loading() {
    println!("=== Testing Patch Loading ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Create minimal test patch
    let mut patch_data = [0u8; 155];
    patch_data[145..155].copy_from_slice(b"TEST PATCH");
    patch_data[134] = 0; // Algorithm 1 (0-based)
    patch_data[135] = 3; // Some feedback

    // Set operator 0 to have reasonable parameters
    patch_data[16] = 50;  // Output level
    patch_data[17] = 0;   // Frequency mode (ratio)
    patch_data[18] = 1;   // Coarse = 1 (1:1 ratio)
    patch_data[19] = 0;   // Fine = 0
    patch_data[20] = 7;   // Detune = 7 (center)

    let patch = dx7tv::sysex::Dx7Patch::from_data(&patch_data).unwrap();
    println!("Created patch: {}", patch.name);

    synth.load_patch(patch).unwrap();
    println!("Loaded patch successfully");

    assert_eq!(synth.current_patch_name(), Some("TEST PATCH"));
    println!("Patch loading: PASS");
}

#[test]
fn test_short_note_generation() {
    println!("=== Testing Short Note Generation ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Create test patch with reasonable parameters
    let mut patch_data = [0u8; 155];
    patch_data[145..155].copy_from_slice(b"TEST NOTE ");
    patch_data[134] = 0; // Algorithm 1
    patch_data[135] = 0; // No feedback initially

    // Set operator 0 (modulator in algorithm 1)
    patch_data[16] = 99;  // Max output level
    patch_data[17] = 0;   // Ratio mode
    patch_data[18] = 1;   // Coarse = 1 (1:1 ratio)
    patch_data[19] = 0;   // Fine = 0
    patch_data[20] = 7;   // Detune = center

    // Set envelope for operator 0
    patch_data[0] = 50;   // Attack rate
    patch_data[1] = 50;   // Decay 1 rate
    patch_data[2] = 50;   // Decay 2 rate
    patch_data[3] = 50;   // Release rate
    patch_data[4] = 99;   // Attack level
    patch_data[5] = 80;   // Decay 1 level
    patch_data[6] = 60;   // Decay 2 level
    patch_data[7] = 0;    // Release level

    // Set operator 3 (carrier in algorithm 1) - starts at byte 63
    patch_data[63+16] = 99;  // Max output level (byte 79)
    patch_data[63+17] = 0;   // Ratio mode
    patch_data[63+18] = 1;   // Coarse = 1 (1:1 ratio)
    patch_data[63+19] = 0;   // Fine = 0
    patch_data[63+20] = 7;   // Detune = center

    // Set envelope for operator 3
    patch_data[63+0] = 50;   // Attack rate (byte 63)
    patch_data[63+1] = 50;   // Decay 1 rate
    patch_data[63+2] = 50;   // Decay 2 rate
    patch_data[63+3] = 50;   // Release rate
    patch_data[63+4] = 99;   // Attack level
    patch_data[63+5] = 80;   // Decay 1 level
    patch_data[63+6] = 60;   // Decay 2 level
    patch_data[63+7] = 0;    // Release level

    let patch = dx7tv::sysex::Dx7Patch::from_data(&patch_data).unwrap();
    synth.load_patch(patch).unwrap();

    // Generate very short note
    let samples = synth.render_note(60, 127, 0.01).unwrap(); // 10ms, max velocity

    println!("Generated {} samples", samples.len());
    println!("First 5 samples: {:?}", &samples[0..5.min(samples.len())]);

    // Check basic properties
    assert!(!samples.is_empty(), "Should generate some samples");
    assert!(samples.len() <= 441 * 2, "Should not exceed expected length"); // 10ms at 44.1kHz with some margin

    // Check if any samples are non-zero
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Non-zero samples: {}/{}", non_zero_count, samples.len());

    if non_zero_count > 0 {
        println!("Note generation: PASS (has audio)");
    } else {
        println!("Note generation: FAIL (silent)");

        // Debug info
        println!("All samples are effectively zero - audio path is broken");
        panic!("Audio generation is silent - this indicates a fundamental issue");
    }
}

#[test]
fn test_star1_fast_decay_preset0_pitch() {
    env_logger::init();
    println!("=== Star1 Fast Decay Preset 0 Pitch Test ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Load the actual star1-fast-decay.syx file
    let syx_data = std::fs::read("star1-fast-decay.syx")
        .expect("Could not read star1-fast-decay.syx - make sure file exists in project root");

    // Parse SYSEX and load preset 0
    let patches = dx7tv::sysex::parse_sysex_data(&syx_data)
        .expect("Failed to parse SYSEX file");

    assert!(!patches.is_empty(), "No patches found in SYSEX file");
    let patch = &patches[0]; // Get preset 0

    println!("Loaded patch: {}", patch.name);
    synth.load_patch(patch.clone()).unwrap();

    // Expected frequency for MIDI note 60 (middle C)
    let expected_freq = 440.0 * 2.0_f64.powf((60.0 - 69.0) / 12.0);
    println!("Expected frequency for MIDI note 60: {:.2} Hz", expected_freq);

    // Generate a short audio sample
    let samples = synth.render_note(60, 127, 0.1).unwrap(); // 100ms, max velocity

    println!("Generated {} samples", samples.len());

    // Check if audio was generated
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Non-zero samples: {}/{}", non_zero_count, samples.len());

    assert!(non_zero_count > 0, "Should generate audio");

    // For debugging: show first few samples
    println!("First 10 samples: {:?}", &samples[0..10.min(samples.len())]);

    // The debug output from the synthesis should show what frequency is actually being calculated
    println!("Check the DEBUG FREQ output above to see if frequency calculation matches {:.2} Hz", expected_freq);

    // Analyze the audio for coherence vs noise
    if samples.len() >= 1000 {
        // Check if the audio shows periodic behavior (sine-like) or is just noise
        let sample_rate = 44100.0;
        let actual_freq = 130.30; // From debug output - coarse=0 gives 0.5x ratio
        let expected_period_samples = sample_rate / actual_freq;

        println!("Expected period: {:.1} samples for {:.2} Hz", expected_period_samples, actual_freq);

        // Compare samples at one period apart - should be similar for sine wave
        let period = expected_period_samples as usize;
        if samples.len() > period * 2 {
            let mut correlation_sum = 0.0;
            let mut sample_count = 0;

            for i in 0..(samples.len() - period) {
                if i + period < samples.len() {
                    correlation_sum += samples[i] * samples[i + period];
                    sample_count += 1;
                }
            }

            let correlation = if sample_count > 0 { correlation_sum / sample_count as f32 } else { 0.0 };
            println!("Period correlation: {:.6} (>0.5 suggests periodic, <0.1 suggests noise)", correlation);

            if correlation < 0.1 {
                println!("WARNING: Audio appears to be noise rather than periodic signal!");
            }
        }

        // Show RMS level
        let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
        println!("RMS level: {:.6}", rms);

        // Show sample distribution
        let max_val = samples.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        println!("Max amplitude: {:.6}", max_val);
    }
}

#[test]
fn test_star1_fast_decay_preset1_pitch() {
    println!("=== Star1 Fast Decay Preset 1 Pitch Test ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Load the actual star1-fast-decay.syx file
    let syx_data = std::fs::read("star1-fast-decay.syx")
        .expect("Could not read star1-fast-decay.syx - make sure file exists in project root");

    // Parse SYSEX and load preset 1
    let patches = dx7tv::sysex::parse_sysex_data(&syx_data)
        .expect("Failed to parse SYSEX file");

    assert!(patches.len() > 1, "Need at least 2 patches for preset 1");
    let patch = &patches[1]; // Get preset 1

    println!("Loaded patch: {}", patch.name);
    synth.load_patch(patch.clone()).unwrap();

    // Expected frequency for MIDI note 60 (middle C)
    let expected_freq = 440.0 * 2.0_f64.powf((60.0 - 69.0) / 12.0);
    println!("Expected frequency for MIDI note 60: {:.2} Hz", expected_freq);

    // Generate a short audio sample
    let samples = synth.render_note(60, 127, 0.1).unwrap(); // 100ms, max velocity

    println!("Generated {} samples", samples.len());

    // Check if audio was generated
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Non-zero samples: {}/{}", non_zero_count, samples.len());

    assert!(non_zero_count > 0, "Should generate audio");

    println!("Check the DEBUG FREQ output above to see if frequency calculation matches {:.2} Hz", expected_freq);
}

#[test]
fn test_star1_fast_decay_preset2_pitch() {
    println!("=== Star1 Fast Decay Preset 2 Pitch Test ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Load the actual star1-fast-decay.syx file
    let syx_data = std::fs::read("star1-fast-decay.syx")
        .expect("Could not read star1-fast-decay.syx - make sure file exists in project root");

    // Parse SYSEX and load preset 2
    let patches = dx7tv::sysex::parse_sysex_data(&syx_data)
        .expect("Failed to parse SYSEX file");

    assert!(patches.len() > 2, "Need at least 3 patches for preset 2");
    let patch = &patches[2]; // Get preset 2

    println!("Loaded patch: {}", patch.name);
    synth.load_patch(patch.clone()).unwrap();

    // Expected frequency for MIDI note 60 (middle C)
    let expected_freq = 440.0 * 2.0_f64.powf((60.0 - 69.0) / 12.0);
    println!("Expected frequency for MIDI note 60: {:.2} Hz", expected_freq);

    // Generate a short audio sample
    let samples = synth.render_note(60, 127, 0.1).unwrap(); // 100ms, max velocity

    println!("Generated {} samples", samples.len());

    // Check if audio was generated
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Non-zero samples: {}/{}", non_zero_count, samples.len());

    assert!(non_zero_count > 0, "Should generate audio");

    println!("Check the DEBUG FREQ output above to see if frequency calculation matches {:.2} Hz", expected_freq);
}

#[test]
fn test_star1_fast_decay_preset3_pitch() {
    println!("=== Star1 Fast Decay Preset 3 Pitch Test ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Load the actual star1-fast-decay.syx file
    let syx_data = std::fs::read("star1-fast-decay.syx")
        .expect("Could not read star1-fast-decay.syx - make sure file exists in project root");

    // Parse SYSEX and load preset 3
    let patches = dx7tv::sysex::parse_sysex_data(&syx_data)
        .expect("Failed to parse SYSEX file");

    assert!(patches.len() > 3, "Need at least 4 patches for preset 3");
    let patch = &patches[3]; // Get preset 3

    println!("Loaded patch: {}", patch.name);
    synth.load_patch(patch.clone()).unwrap();

    // Expected frequency for MIDI note 60 (middle C)
    let expected_freq = 440.0 * 2.0_f64.powf((60.0 - 69.0) / 12.0);
    println!("Expected frequency for MIDI note 60: {:.2} Hz", expected_freq);

    // Generate a short audio sample
    let samples = synth.render_note(60, 127, 0.1).unwrap(); // 100ms, max velocity

    println!("Generated {} samples", samples.len());

    // Check if audio was generated
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Non-zero samples: {}/{}", non_zero_count, samples.len());

    assert!(non_zero_count > 0, "Should generate audio");

    println!("Check the DEBUG FREQ output above to see if frequency calculation matches {:.2} Hz", expected_freq);
}

#[test]
fn test_single_operator_sine() {
    println!("=== Single Operator Sine Test ===");

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    // Create test patch using structured API
    let mut patch = dx7tv::sysex::Dx7Patch::new("SINE TEST");

    // Set algorithm 1 (stored as 0)
    patch.global.algorithm = 0;
    patch.global.feedback = 0;

    // Configure operator 0 as a simple sine wave carrier
    patch.operators[0].rates.attack = 50;
    patch.operators[0].rates.decay1 = 50;
    patch.operators[0].rates.decay2 = 50;
    patch.operators[0].rates.release = 30;

    patch.operators[0].levels.attack = 99;
    patch.operators[0].levels.decay1 = 90;
    patch.operators[0].levels.decay2 = 80;
    patch.operators[0].levels.release = 0;
    patch.operators[0].output_level = 80;             // Output level
    patch.operators[0].coarse_freq = 1;               // 1:1 frequency ratio
    patch.operators[0].fine_freq = 0;                 // No fine tuning
    patch.operators[0].detune = 7;                    // Center detune

    synth.load_patch(patch).unwrap();

    // Generate audio
    let samples = synth.render_note(60, 127, 0.05).unwrap(); // 50ms
    println!("Generated {} samples", samples.len());

    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Non-zero samples: {}/{}", non_zero_count, samples.len());

    if non_zero_count > 0 {
        // Show first few samples
        println!("First 10 samples: {:?}", &samples[0..10.min(samples.len())]);

        // Check periodicity
        if samples.len() >= 1000 {
            let expected_freq = 261.63; // Should be middle C with coarse=1
            let sample_rate = 44100.0;
            let expected_period = sample_rate / expected_freq;

            let period = expected_period as usize;
            if samples.len() > period * 2 {
                let mut correlation_sum = 0.0;
                let sample_count = samples.len() - period;

                for i in 0..sample_count {
                    correlation_sum += samples[i] * samples[i + period];
                }

                let correlation = correlation_sum / sample_count as f32;
                println!("Period correlation: {:.6} (expected >0.5 for clean sine)", correlation);

                if correlation > 0.5 {
                    println!("SUCCESS: Clean periodic signal detected!");
                } else {
                    println!("WARNING: Signal is not cleanly periodic");
                }
            }
        }
    } else {
        println!("ERROR: No audio generated");
    }
}

#[test]
fn test_expected_vs_actual() {
    println!("=== Expected vs Actual Comparison ===");

    // What we expect for a 261.63 Hz sine wave
    let expected_freq = 261.63;
    let sample_rate = 44100.0;

    println!("Expected frequency: {:.2} Hz", expected_freq);
    println!("Sample rate: {} Hz", sample_rate);

    // Calculate first few samples of expected sine wave
    println!("Expected sine wave samples:");
    for i in 0..5 {
        let t = i as f64 / sample_rate;
        let expected = 0.5 * (2.0 * PI * expected_freq * t).sin();
        println!("Sample {}: {:.6}", i, expected);
    }

    println!();
    println!("Now run the actual synthesis and compare:");
    println!("The actual samples should follow this sine wave pattern");
}