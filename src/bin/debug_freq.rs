use dx7tv::{render_patch, Dx7Patch};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Create a simple sine wave patch
    let mut patch = Dx7Patch::new("FREQ TEST");

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

    // Render MIDI note 60 (C4, ~261.63 Hz)
    let samples = render_patch(patch, 60, 1.0)?;

    log::info!("Generated {} samples", samples.len());
    log::info!("First 10 samples: {:?}", &samples[..10]);

    // Calculate RMS level
    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    log::info!("RMS level: {}", rms);

    Ok(())
}