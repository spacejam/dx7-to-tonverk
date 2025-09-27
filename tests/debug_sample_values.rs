use dx7tv::{sysex, Dx7Synth};
use anyhow::Result;

#[test]
fn debug_sample_values() -> Result<()> {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    // Load star1 patch
    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")?;
    let patch = patches.get(0).unwrap().clone();

    println!("Testing patch: '{}'", patch.name);

    let mut synth = Dx7Synth::new(44100.0, 1.0);
    synth.load_patch(patch)?;

    // Render a very short sample to see the i32 values
    let samples = synth.render_note(60, 100, 0.01)?; // Just 10ms

    println!("Generated {} samples", samples.len());

    // Show first few samples and find peak
    let mut max_abs = 0.0f32;
    for (i, &sample) in samples.iter().take(20).enumerate() {
        println!("Sample {}: {:.6}", i, sample);
        max_abs = max_abs.max(sample.abs());
    }

    let overall_peak = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    println!("Peak amplitude in {} samples: {:.6}", samples.len(), overall_peak);

    // Estimate what i32 values would produce this
    let estimated_i32_peak = overall_peak * 32768.0;
    println!("Estimated i32 peak value: {:.0}", estimated_i32_peak);

    Ok(())
}