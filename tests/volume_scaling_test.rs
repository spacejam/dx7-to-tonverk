use dx7tv::sysex;
use dx7tv::synth::Dx7Synth;

#[test]
fn test_volume_scaling_investigation() {
    let _ = env_logger::try_init();

    println!("=== Investigating Volume Scaling Issue ===");

    // Load the sysex file
    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read and parse star1-fast-decay.syx");
    let patch = patches.get(0).expect("No patches found").clone();

    println!("Testing patch: '{}'", patch.name);

    // Create synth and render audio
    let mut synth = Dx7Synth::new(44100.0, 1.0);
    synth.load_patch(patch).expect("Failed to load patch");

    // Render a short note to examine scaling
    let samples = synth.render_note(60, 127, 0.1).expect("Failed to render note"); // Max velocity

    // Analyze the output
    let max_amplitude = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();

    println!("Current scaling results:");
    println!("  Peak amplitude: {:.8}", max_amplitude);
    println!("  RMS level: {:.8}", rms);
    println!("  Peak as 16-bit PCM: {:.1}", max_amplitude * 32767.0);
    println!("  Expected audible range: 1000-16000 (16-bit PCM)");

    // Test different scaling factors
    let current_divisor = 1i32 << 23; // 8388608
    println!("\nCurrent divisor: {} (2^23)", current_divisor);

    // Test what different divisors would produce
    let test_sample_value = 19652i32; // From debug output

    println!("\nScaling analysis for sample value {}:", test_sample_value);
    for shift in 16..24 {
        let divisor = 1i32 << shift;
        let f32_sample = test_sample_value as f32 / divisor as f32;
        let pcm_16bit = (f32_sample * 32767.0) as i16;
        println!("  2^{:2} ({:8}): f32={:.6}, 16-bit PCM={:5}",
                shift, divisor, f32_sample, pcm_16bit);
    }

    // The issue is likely that the scaling is too conservative
    // Typical 16-bit PCM should use values in thousands for audible sound
    // Our current max of ~139 PCM is barely audible

    println!("\nConclusion:");
    if max_amplitude * 32767.0 < 1000.0 {
        println!("  Audio is too quiet - scaling factor is too large");
        println!("  Current 16-bit PCM peak: {:.1}", max_amplitude * 32767.0);
        println!("  Recommended minimum: 1000-3000 for clearly audible sound");
    } else {
        println!("  Volume scaling appears reasonable");
    }
}

#[test]
fn test_different_velocity_levels() {
    let _ = env_logger::try_init();

    println!("=== Testing Different Velocity Levels ===");

    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read and parse star1-fast-decay.syx");
    let patch = patches.get(0).expect("No patches found").clone();

    let mut synth = Dx7Synth::new(44100.0, 1.0);
    synth.load_patch(patch).expect("Failed to load patch");

    for velocity in [40, 80, 100, 127] {
        let samples = synth.render_note(60, velocity, 0.1).expect("Failed to render note");
        let max_amplitude = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        let pcm_peak = (max_amplitude * 32767.0) as i16;

        println!("Velocity {:3}: peak amplitude={:.6}, 16-bit PCM={:5}",
                velocity, max_amplitude, pcm_peak);
    }
}