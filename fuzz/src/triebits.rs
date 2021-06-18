#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|vecs: triebits::Vecs| {
    triebits::exercise_vecs(vecs);
});
