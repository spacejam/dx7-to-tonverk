use dx7tv::sysex;
use dx7tv::synth::Dx7Synth;

/// Debug test to examine exactly what's happening in algorithm processing

#[test]
fn test_algorithm_4_debug() {
    let _ = env_logger::try_init();

    println!("=== Algorithm 4 Debug Analysis ===");

    // Load the actual star1-fast-decay patch
    let patches = sysex::parse_sysex_file("./star1-fast-decay.syx")
        .expect("Failed to read star1-fast-decay.syx");
    let patch = patches.get(0).expect("No patches found").clone();

    println!("Patch name: '{}'", patch.name);
    println!("Patch algorithm: {} (stored as {})", patch.global.algorithm + 1, patch.global.algorithm);
    println!("Patch feedback: {}", patch.global.feedback);

    // Create a simplified test with algorithm 4 to see if it's the same issue
    let mut simple_patch = dx7tv::sysex::Dx7Patch::new("SIMPLE4");
    simple_patch.global.algorithm = 3; // Algorithm 4 (0-indexed)

    // Set up operators with very basic parameters
    for i in 0..6 {
        simple_patch.operators[i].output_level = 80;
        simple_patch.operators[i].coarse_freq = 1;
        simple_patch.operators[i].fine_freq = 0;
        simple_patch.operators[i].detune = 7; // center
        simple_patch.operators[i].rates.attack = 99;
        simple_patch.operators[i].rates.decay1 = 70;
        simple_patch.operators[i].rates.decay2 = 50;
        simple_patch.operators[i].rates.release = 30;
        simple_patch.operators[i].levels.attack = 99;
        simple_patch.operators[i].levels.decay1 = 90;
        simple_patch.operators[i].levels.decay2 = 80;
        simple_patch.operators[i].levels.release = 0;
    }

    let mut synth = Dx7Synth::new(44100.0, 1.0);

    println!("\n=== Testing Original star1-fast-decay Patch ===");
    synth.load_patch(patch.clone()).expect("Failed to load original patch");
    let original_samples = synth.render_note(60, 100, 0.1).expect("Failed to render original");

    let original_dc = calculate_dc_offset(&original_samples);
    let original_rms = calculate_rms(&original_samples);
    let original_peak = original_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Original patch stats:");
    println!("  DC offset: {:.8}", original_dc);
    println!("  RMS: {:.6}", original_rms);
    println!("  Peak: {:.6}", original_peak);
    println!("  Sample range: {:.6} to {:.6}",
             original_samples.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
             original_samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)));

    println!("\n=== Testing Simplified Algorithm 4 Patch ===");
    synth.load_patch(simple_patch).expect("Failed to load simple patch");
    let simple_samples = synth.render_note(60, 100, 0.1).expect("Failed to render simple");

    let simple_dc = calculate_dc_offset(&simple_samples);
    let simple_rms = calculate_rms(&simple_samples);
    let simple_peak = simple_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Simple patch stats:");
    println!("  DC offset: {:.8}", simple_dc);
    println!("  RMS: {:.6}", simple_rms);
    println!("  Peak: {:.6}", simple_peak);
    println!("  Sample range: {:.6} to {:.6}",
             simple_samples.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
             simple_samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)));

    // Compare with a simple algorithm (algorithm 32 - all parallel)
    println!("\n=== Testing Algorithm 32 (All Parallel) for Comparison ===");
    let mut parallel_patch = dx7tv::sysex::Dx7Patch::new("PARALLEL");
    parallel_patch.global.algorithm = 31; // Algorithm 32 (0-indexed)

    // Only enable one operator to keep it simple
    parallel_patch.operators[0].output_level = 80;
    parallel_patch.operators[0].coarse_freq = 1;
    parallel_patch.operators[0].rates.attack = 99;
    parallel_patch.operators[0].levels.attack = 99;
    for i in 1..6 {
        parallel_patch.operators[i].output_level = 0; // Disable other operators
    }

    synth.load_patch(parallel_patch).expect("Failed to load parallel patch");
    let parallel_samples = synth.render_note(60, 100, 0.1).expect("Failed to render parallel");

    let parallel_dc = calculate_dc_offset(&parallel_samples);
    let parallel_rms = calculate_rms(&parallel_samples);
    let parallel_peak = parallel_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Parallel patch stats:");
    println!("  DC offset: {:.8}", parallel_dc);
    println!("  RMS: {:.6}", parallel_rms);
    println!("  Peak: {:.6}", parallel_peak);
    println!("  Sample range: {:.6} to {:.6}",
             parallel_samples.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
             parallel_samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)));

    // Analysis
    println!("\n=== Analysis ===");

    if original_dc.abs() > 0.01 {
        println!("⚠️  Original patch has significant DC offset: {:.6}", original_dc);
    }

    if simple_dc.abs() > 0.01 {
        println!("⚠️  Simple algorithm 4 patch has DC offset: {:.6}", simple_dc);
    }

    if parallel_dc.abs() > 0.01 {
        println!("⚠️  Even simple parallel patch has DC offset: {:.6}", parallel_dc);
    } else {
        println!("✅ Parallel patch has minimal DC offset: {:.6}", parallel_dc);
    }

    // Check if the issue is specific to algorithm 4 or more general
    if simple_dc.abs() > parallel_dc.abs() * 5.0 {
        println!("🔍 Algorithm 4 has significantly more DC offset than simple parallel");
        println!("   This suggests the issue is in the FM algorithm routing");
    } else {
        println!("🔍 DC offset is similar across algorithms");
        println!("   The issue might be in the synthesis engine itself");
    }

    println!("\n✅ Algorithm debug test completed");
}

fn calculate_dc_offset(samples: &[f32]) -> f64 {
    samples.iter().map(|&x| x as f64).sum::<f64>() / samples.len() as f64
}

fn calculate_rms(samples: &[f32]) -> f64 {
    let mean_square = samples.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / samples.len() as f64;
    mean_square.sqrt()
}