use dx7tv::fm::freqlut::Freqlut;

#[test]
fn debug_phase_relationship() {
    Freqlut::init(44100.0);

    // Test with known values - MIDI note 60 (C4 = 261.63 Hz)
    let base = 50857777;
    let step = 1398101;
    let midinote = 60;
    let logfreq = base + step * midinote;
    let phase_inc = Freqlut::lookup(logfreq);

    println!("=== PHASE INCREMENT ANALYSIS ===");
    println!("MIDI note: {}", midinote);
    println!("Expected freq: {:.2} Hz", 440.0 * 2.0_f64.powf((midinote - 69) as f64 / 12.0));
    println!("logfreq: {}", logfreq);
    println!("phase_inc: {} (0x{:08x})", phase_inc, phase_inc);
    println!("phase_inc binary: {:032b}", phase_inc as u32);

    // Test different interpretations of phase increment
    let sample_rate = 44100.0;

    // Interpretation 1: 32-bit phase accumulator (2^32 represents one full cycle)
    let freq1 = (phase_inc as f64 * sample_rate) / (1u64 << 32) as f64;
    println!("Interpretation 1 (32-bit): {:.3} Hz", freq1);

    // Interpretation 2: 24-bit phase accumulator (2^24 represents one full cycle)
    let freq2 = (phase_inc as f64 * sample_rate) / (1u64 << 24) as f64;
    println!("Interpretation 2 (24-bit): {:.3} Hz", freq2);

    // Interpretation 3: Dexed might use different scaling
    // Let's try to reverse-engineer from expected frequency
    let expected_freq = 261.63;
    let required_scale = expected_freq / ((phase_inc as f64 * sample_rate) / (1u64 << 32) as f64);
    println!("Required scale factor: {:.3}", required_scale);
    println!("Required scale as power of 2: ~{:.1}", required_scale.log2());

    // Test multiple notes to see if pattern is consistent
    println!("\n=== MULTIPLE NOTE TEST ===");
    for note in [48, 60, 72, 84] {
        let logfreq = base + step * note;
        let phase_inc = Freqlut::lookup(logfreq);
        let expected = 440.0 * 2.0_f64.powf((note - 69) as f64 / 12.0);

        // Try different scaling factors based on powers of 2
        let freq_no_scale = (phase_inc as f64 * sample_rate) / (1u64 << 32) as f64;
        let freq_div32 = freq_no_scale * 32.0; // Our empirical finding

        println!("MIDI {}: phase_inc={}, expected={:.2}Hz, unscaled={:.3}Hz, *32={:.2}Hz, ratio={:.1}",
                 note, phase_inc, expected, freq_no_scale, freq_div32, freq_div32 / expected);
    }

    // Check if there's a pattern in the bit representation
    println!("\n=== BIT PATTERN ANALYSIS ===");
    let phase_inc_u32 = phase_inc as u32;
    for shift in 0..16 {
        let shifted = phase_inc >> shift;
        let freq = (shifted as f64 * sample_rate) / (1u64 << 32) as f64;
        println!(">>{}:  value={:8}, freq={:8.2}Hz, ratio to 261.63Hz = {:.3}",
                 shift, shifted, freq, freq / 261.63);

        // Highlight the one closest to expected
        if (freq / 261.63 - 1.0).abs() < 0.1 {
            println!("      ^^^^ CLOSEST MATCH");
        }
    }
}