use std::io::Write;

use hound::{WavSpec, WavWriter};

use dx7::fm::{
    patch::{Patch, PatchBank},
    voice::{Parameters, Voice},
};

mod common;
use common::generate_wav;

#[test]
fn smoke_test() {
    const SAMPLE_RATE: f32 = 44100.0;

    let patch_bank_bytes =
        std::fs::read("star1-fast-decay.syx").expect("test file star1-fast-decay.syx not found");

    let patch_bank = PatchBank::new(&patch_bank_bytes);

    for (patch_idx, expected_algo, expected_feedback) in [(0, 3, 0), (20, 2, 3), (31, 4, 7)] {
        assert_eq!(
            patch_bank.patches[patch_idx].algorithm, expected_algo,
            "parsed patch does not match expected algorithm, patch idx: {}, data: {:#?}",
            patch_idx, patch_bank.patches[patch_idx]
        );
    }

    for patch_number in [11] {
        let patch = patch_bank.patches[patch_number];

        let wav_data = generate_wav(patch, 60.0, 44100, std::time::Duration::from_secs(2));

        let file_name = format!("smoke-{}.wav", patch.name.iter().collect::<String>().trim());
        let mut file = std::fs::File::create(file_name).unwrap();
        file.write_all(&wav_data).unwrap();
        file.sync_all().unwrap();
    }
}
