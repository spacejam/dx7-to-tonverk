use dx7tv::sysex::parse_sysex_file;
use anyhow::Result;

#[test]
fn debug_patch20_algorithm() -> Result<()> {
    let patches = parse_sysex_file("star1-fast-decay.syx")?;
    let patch_20 = &patches[20];

    println!("=== PATCH 20 ANALYSIS ===");
    println!("Patch name: '{}'", patch_20.name);
    println!("Algorithm: {}", patch_20.global.algorithm);

    // Print all operator configurations
    for (i, op) in patch_20.operators.iter().enumerate() {
        println!("\n--- OPERATOR {} ---", i);
        println!("Output level: {}", op.output_level);
        println!("Coarse freq: {}", op.coarse_freq);
        println!("Fine freq: {}", op.fine_freq);
        println!("Detune: {}", op.detune);
        println!("Osc mode: {} ({})", op.osc_mode, if op.osc_mode == 0 { "Ratio" } else { "Fixed" });

        println!("Attack rate: {}, level: {}", op.rates.attack, op.levels.attack);
        println!("Decay1 rate: {}, level: {}", op.rates.decay1, op.levels.decay1);
        println!("Decay2 rate: {}, level: {}", op.rates.decay2, op.levels.decay2);
        println!("Release rate: {}, level: {}", op.rates.release, op.levels.release);

        // Check if operator is effectively silent
        let is_silent = op.output_level == 0 ||
                       (op.levels.attack == 0 && op.levels.decay1 == 0 && op.levels.decay2 == 0);
        println!("Effectively silent: {}", is_silent);
    }

    println!("\n=== ALGORITHM {} ROUTING ===", patch_20.global.algorithm);
    match patch_20.global.algorithm {
        0 => println!("Algorithm 1: [6->5], carriers: [5]"),
        1 => println!("Algorithm 2: [6->5, 4->5], carriers: [5]"),
        2 => println!("Algorithm 3: [6->5, 4->3->2->1], carriers: [5,1]"),
        3 => println!("Algorithm 4: [6->5, 4->3], carriers: [5,3,2,1]"),
        // Add more algorithms as needed
        _ => println!("Algorithm {}: Details not mapped", patch_20.global.algorithm),
    }

    println!("\n=== GLOBAL PARAMETERS ===");
    println!("Pitch EG Rate: {:?}", patch_20.global.pitch_eg_rate);
    println!("Pitch EG Level: {:?}", patch_20.global.pitch_eg_level);
    println!("Feedback: {}", patch_20.global.feedback);
    println!("Osc Sync: {}", patch_20.global.osc_sync);

    Ok(())
}