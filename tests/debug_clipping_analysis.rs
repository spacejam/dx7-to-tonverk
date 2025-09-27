use dx7tv::{Dx7Synth, sysex::parse_sysex_file};
use anyhow::Result;

#[test]
fn debug_clipping_analysis() -> Result<()> {
    // Generate synthesized audio
    let patches = parse_sysex_file("star1-fast-decay.syx")?;
    let patch_20 = &patches[20];

    let mut synth = Dx7Synth::new(44100.0, 4.0);
    synth.load_patch(patch_20.clone())?;
    let samples = synth.render_note(60, 127, 4.0)?;

    println!("=== CLIPPING ANALYSIS ===");
    println!("Total samples: {}", samples.len());

    // Find extreme values
    let mut max_val = f32::MIN;
    let mut min_val = f32::MAX;
    let mut max_abs = 0.0f32;
    let mut clipped_samples = 0;
    let mut near_clipped = 0;

    for (i, &sample) in samples.iter().enumerate() {
        if sample > max_val { max_val = sample; }
        if sample < min_val { min_val = sample; }
        let abs_val = sample.abs();
        if abs_val > max_abs { max_abs = abs_val; }

        // Check for clipping (values at or near +/-1.0)
        if abs_val >= 1.0 {
            clipped_samples += 1;
            if i < 10 {
                println!("CLIPPED sample at index {}: {}", i, sample);
            }
        } else if abs_val > 0.95 {
            near_clipped += 1;
        }
    }

    println!("Max value: {}", max_val);
    println!("Min value: {}", min_val);
    println!("Max absolute: {}", max_abs);
    println!("Clipped samples (>=1.0): {} ({:.2}%)",
             clipped_samples, 100.0 * clipped_samples as f32 / samples.len() as f32);
    println!("Near-clipped samples (>0.95): {} ({:.2}%)",
             near_clipped, 100.0 * near_clipped as f32 / samples.len() as f32);

    // Analyze dynamic range
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
    let peak_to_rms = max_abs / rms;
    println!("RMS: {:.6}", rms);
    println!("Peak-to-RMS ratio: {:.2} dB", 20.0 * peak_to_rms.log10());

    // Check for sudden amplitude jumps that might indicate artifacts
    let mut large_jumps = 0;
    for i in 1..samples.len() {
        let jump = (samples[i] - samples[i-1]).abs();
        if jump > 0.1 {  // Arbitrary threshold for "large jump"
            large_jumps += 1;
            if large_jumps <= 5 {
                println!("Large amplitude jump at {}: {} -> {} (diff: {})",
                         i, samples[i-1], samples[i], jump);
            }
        }
    }
    println!("Large amplitude jumps (>0.1): {}", large_jumps);

    // Sample some values at different time points
    println!("\n=== SAMPLE VALUES OVER TIME ===");
    for t in [0.0, 0.1, 0.5, 1.0, 2.0, 3.0] {
        let index = (t * 44100.0) as usize;
        if index < samples.len() {
            println!("t={:.1}s (sample {}): {:.6}", t, index, samples[index]);
        }
    }

    Ok(())
}