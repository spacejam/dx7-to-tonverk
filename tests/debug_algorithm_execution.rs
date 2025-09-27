use dx7tv::{Dx7Synth, sysex::parse_sysex_file};
use anyhow::Result;

#[test]
fn debug_algorithm_execution() -> Result<()> {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    println!("=== ALGORITHM EXECUTION DEBUG ===");

    let patches = parse_sysex_file("star1-fast-decay.syx")?;
    let patch_20 = &patches[20];

    println!("Testing patch: '{}', Algorithm: {}", patch_20.name, patch_20.global.algorithm);

    let mut synth = Dx7Synth::new(44100.0, 0.1); // Very short duration
    synth.load_patch(patch_20.clone())?;

    // Generate just a few samples to see the debug output
    let samples = synth.render_note(60, 127, 0.01)?; // 10ms

    println!("Generated {} samples", samples.len());

    // Show first few samples
    for (i, &sample) in samples.iter().take(20).enumerate() {
        if sample.abs() > 1e-8 {
            println!("Sample {}: {:.8}", i, sample);
        }
    }

    // Check for signs of noise vs. proper synthesis
    let non_zero_count = samples.iter().filter(|&&x| x.abs() > 1e-8).count();
    let max_sample = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();

    println!("Non-zero samples: {} / {}", non_zero_count, samples.len());
    println!("Max sample: {:.8}", max_sample);
    println!("RMS: {:.8}", rms);

    // Quick spectral analysis - look for DC bias or obvious noise patterns
    let mut dc_component = samples.iter().sum::<f32>() / samples.len() as f32;
    println!("DC component: {:.8}", dc_component);

    Ok(())
}