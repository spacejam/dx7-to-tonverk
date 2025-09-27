use anyhow::Result;
use hound::WavReader;

#[test]
fn debug_patch20_comparison() -> Result<()> {
    // Load both files for direct comparison
    let reference_samples = load_wav_file("patch-20-c3-2s.wav")?;
    let generated_samples = load_wav_file("out/debug-patch20.wav")?;

    println!("Reference file: {} samples", reference_samples.len());
    println!("Generated file: {} samples", generated_samples.len());

    // Show first few samples
    println!("Reference samples [0-9]: {:?}", &reference_samples[..10.min(reference_samples.len())]);
    println!("Generated samples [0-9]: {:?}", &generated_samples[..10.min(generated_samples.len())]);

    // Calculate basic stats for both
    let ref_rms = (reference_samples.iter().map(|&x| x * x).sum::<f32>() / reference_samples.len() as f32).sqrt();
    let ref_peak = reference_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    let gen_rms = (generated_samples.iter().map(|&x| x * x).sum::<f32>() / generated_samples.len() as f32).sqrt();
    let gen_peak = generated_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Reference: RMS={:.6}, Peak={:.6}", ref_rms, ref_peak);
    println!("Generated: RMS={:.6}, Peak={:.6}", gen_rms, gen_peak);

    // Check if generated file has proper audio content
    let non_zero_count = generated_samples.iter().filter(|&&x| x.abs() > 1e-6).count();
    println!("Generated non-zero samples: {} / {}", non_zero_count, generated_samples.len());

    Ok(())
}

fn load_wav_file(filename: &str) -> Result<Vec<f32>> {
    let mut reader = WavReader::open(filename)?;
    let spec = reader.spec();

    let samples: Vec<f32> = match spec.bits_per_sample {
        16 => {
            reader.samples::<i16>()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|s| s as f32 / 32768.0) // Convert i16 to f32
                .collect()
        }
        32 => {
            reader.samples::<f32>()
                .collect::<Result<Vec<_>, _>>()?
        }
        _ => return Err(anyhow::anyhow!("Unsupported bit depth: {}", spec.bits_per_sample))
    };

    Ok(samples)
}