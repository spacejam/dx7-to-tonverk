use dx7tv::fm::freqlut::Freqlut;

#[test]
fn debug_freqlut_values() {
    // Initialize Freqlut with same sample rate as Dexed
    Freqlut::init(44100.0);

    // Test MIDI note 60 (middle C) logfreq calculation
    // Using Dexed's standard tuning: base = 50857777, step = 1398101
    let base = 50857777;
    let step = 1398101; // (1 << 24) / 12
    let midinote = 60;
    let logfreq = base + step * midinote;

    println!("MIDI note {}: logfreq = {}", midinote, logfreq);

    // Get phase increment from Freqlut
    let phase_inc = Freqlut::lookup(logfreq);
    println!("Freqlut::lookup({}) = {}", logfreq, phase_inc);

    // Calculate what frequency this should produce
    // For 44100 Hz sample rate, phase increment relates to frequency as:
    // freq_hz = (phase_inc * sample_rate) / (1 << 32)
    let freq_hz_calc = (phase_inc as f64 * 44100.0) / (1u64 << 32) as f64;
    println!("Calculated frequency: {:.2} Hz", freq_hz_calc);

    // Expected frequency for MIDI note 60 (middle C)
    let expected_freq = 440.0 * 2.0_f64.powf((60 - 69) as f64 / 12.0);
    println!("Expected C4 frequency: {:.2} Hz", expected_freq);
    println!("Ratio: {:.2}x", freq_hz_calc / expected_freq);

    // Test a few more MIDI notes
    for note in [48, 60, 72] {
        let logfreq = base + step * note;
        let phase_inc = Freqlut::lookup(logfreq);
        let freq_hz = (phase_inc as f64 * 44100.0) / (1u64 << 32) as f64;
        let expected = 440.0 * 2.0_f64.powf((note - 69) as f64 / 12.0);
        println!("MIDI {}: logfreq={}, phase_inc={}, freq={:.2}Hz, expected={:.2}Hz, ratio={:.2}x",
                 note, logfreq, phase_inc, freq_hz, expected, freq_hz / expected);
    }
}