use dx7tv::{parse_sysex_file, render_patch_to_wav};
use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();

    // Load a patch from the SYSEX file
    let patches = parse_sysex_file("star1-fast-decay.syx")?;
    let patch = &patches[0]; // Use first patch

    println!("Testing patch: {}", patch.name);

    // Render a simple C4 note for 2 seconds
    render_patch_to_wav(patch.clone(), 60, 2.0, "out/test_new_api.wav")?;

    println!("Generated: out/test_new_api.wav");
    Ok(())
}