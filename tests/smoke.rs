use hound::{WavSpec, WavWriter};

use dx7::fm::{
    patch::{Patch, PatchBank},
    voice::{Parameters, Voice},
};

mod common;
use common::wav_from_patch;

#[test]
fn smoke_test() {
    const SAMPLE_RATE: f32 = 44100.0;
    const PATCH_NUMBER: usize = 0;

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

    let patch = patch_bank.patches[PATCH_NUMBER];
    dbg!(&patch);

    let wav_data = wav_from_patch(patch, 60.0, 44100, std::time::Duration::from_secs(2));
}
