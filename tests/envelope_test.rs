use dx7::fm::patch::{OpEnvelope, Operator, Patch};
use dx7::fm::voice::{Parameters, Voice};

#[test]
fn test_envelope_triggering() {
    let mut patch = Patch::default();

    // Create a simple operator with fast attack, no decay/sustain drop, instant release
    let operator = Operator {
        envelope: OpEnvelope {
            rate: [99, 99, 99, 99],   // Fast everything
            level: [99, 99, 99, 0],    // Full level, then to 0 on release
        },
        level: 99,
        coarse: 1,
        fine: 0,
        ..Operator::default()
    };

    patch.set_op(1, operator);
    patch.algorithm = 31; // Simple algorithm with one carrier

    let sample_rate = 44100;
    let mut voice = Voice::new(patch, sample_rate as f32);

    // Render with gate OFF first (should be silent)
    let mut buf1 = vec![0.0f32; 300]; // 3x100 for render_temp
    let params_off = Parameters {
        gate: false,
        sustain: false,
        velocity: 1.0,
        note: 69.0,
        ..Parameters::default()
    };
    voice.render_temp(&params_off, &mut buf1);
    let max_off = buf1.iter().take(100).map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude with gate OFF: {}", max_off);

    // Render with gate ON (should trigger envelope)
    let mut buf2 = vec![0.0f32; 300];
    let params_on = Parameters {
        gate: true,
        sustain: false,
        velocity: 1.0,
        note: 69.0,
        ..Parameters::default()
    };
    voice.render_temp(&params_on, &mut buf2);
    let max_on = buf2.iter().take(100).map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude with gate ON: {}", max_on);

    // Check envelope level directly
    println!("Operator 0 level after gate ON: {}", voice.op_level(0));

    // The gate ON render should have significantly more amplitude
    assert!(max_on > 0.01, "Gate ON should produce audible output, got {}", max_on);
    assert!(max_on > max_off * 10.0, "Gate ON should be much louder than gate OFF");
}
