use dx7tv::{sysex::parse_sysex_data, Dx7Synth};

#[test]
fn debug_polyphony_amplitude() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    // Try to use star1 instead of ROM1A since that's available
    let sysex_paths = [
        "ROM1A.syx",
        "star1-fast-decay.syx",
    ];

    let mut patches = Vec::new();
    for path in &sysex_paths {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(parsed_patches) = parse_sysex_data(&data) {
                patches = parsed_patches;
                println!("Using sysex file: {}", path);
                break;
            }
        }
    }

    if patches.is_empty() {
        println!("No sysex file found, skipping test");
        return;
    }

    let patch = &patches[0];
    let mut synth = Dx7Synth::new(44100.0, 5.0);
    synth.load_patch(patch.clone()).expect("Failed to load patch");

    // Test multiple overlapping notes - same as original test
    let notes = [60, 64, 67]; // C major chord
    let mut all_samples = Vec::new();

    for &note in &notes {
        println!("Rendering note: {}", note);
        let samples = synth.render_note(note, 100, 0.5).expect("Failed to render note");

        // Show stats for this note
        let note_max = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        let note_rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
        println!("  Note {} - samples: {}, max_amp: {:.6}, rms: {:.6}",
                 note, samples.len(), note_max, note_rms);

        all_samples.extend(samples);
    }

    // Calculate overall stats
    let rms = (all_samples.iter().map(|&x| x * x).sum::<f32>() / all_samples.len() as f32).sqrt();
    let max_amp = all_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall - total samples: {}, RMS: {:.6}, max_amp: {:.6}",
             all_samples.len(), rms, max_amp);

    // Check what the original test expects
    println!("Original test checks:");
    println!("  rms > 0.001: {} (actual: {:.6})", rms > 0.001, rms);
    println!("  max_amp < 20.0: {} (actual: {:.6})", max_amp < 20.0, max_amp);
    println!("  all finite: {}", all_samples.iter().all(|&x| x.is_finite()));
}