use dx7tv::{render_patch, Dx7Patch};
use dx7tv::fm::ref_freq;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Test the frequency calculation directly
    println!("=== Direct Reference Frequency Calculation ===");
    let sample_rate = 48000.0;
    let midi_note = 60; // C4
    let base_freq = ref_freq::base_frequency(midi_note, sample_rate, 0.0);
    let ratio = ref_freq::frequency_ratio(0, 1, 0, 7); // 1:1 ratio, center detune
    let one_hz = 1.0 / sample_rate as f32;
    let op_freq_hz = ref_freq::operator_frequency(ratio, base_freq, one_hz);

    println!("MIDI note: {}", midi_note);
    println!("Sample rate: {} Hz", sample_rate);
    println!("Base frequency: {} Hz", base_freq);
    println!("Frequency ratio: {}", ratio);
    println!("One Hz: {}", one_hz);
    println!("Operator frequency: {} Hz", op_freq_hz);
    println!("Expected C4 frequency: ~261.63 Hz");

    // Convert to actual frequency in Hz
    let actual_freq_hz = op_freq_hz * sample_rate as f32;
    println!("Actual frequency in Hz: {} Hz", actual_freq_hz);

    // Convert to 32-bit phase increment (matching reference)
    let phase_inc_32bit = (op_freq_hz.min(0.5) * ((1u64 << 32) as f32)) as i32;
    println!("32-bit phase increment: {}", phase_inc_32bit);

    // Convert phase increment back to frequency to verify
    let freq_from_phase_inc = (phase_inc_32bit as f32) / ((1u64 << 32) as f32) * sample_rate as f32;
    println!("Frequency from phase inc: {} Hz", freq_from_phase_inc);

    println!("\n=== Full Synthesis Test ===");

    // Create a simple sine wave patch
    let mut patch = Dx7Patch::new("FREQ_TEST");
    patch.operators[0].output_level = 99;
    patch.operators[0].rates.attack = 99;
    patch.operators[0].rates.decay1 = 99;
    patch.operators[0].rates.decay2 = 99;
    patch.operators[0].rates.release = 0;
    patch.operators[0].levels.attack = 99;
    patch.operators[0].levels.decay1 = 99;
    patch.operators[0].levels.decay2 = 99;
    patch.operators[0].levels.release = 0;
    patch.operators[0].coarse_freq = 1;  // 1:1 frequency ratio
    patch.operators[0].fine_freq = 0;    // No fine tuning
    patch.operators[0].detune = 7;       // Center detune
    patch.global.algorithm = 0;          // Algorithm 1: OP0 as carrier

    // Render MIDI note 60 (C4, ~261.63 Hz) - short duration for debugging
    let samples = render_patch(patch, 60, 0.1)?;
    println!("Generated {} samples", samples.len());

    // Show first few samples
    println!("First 20 samples: {:?}", &samples[..20.min(samples.len())]);

    // Calculate RMS
    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    println!("RMS level: {}", rms);

    Ok(())
}