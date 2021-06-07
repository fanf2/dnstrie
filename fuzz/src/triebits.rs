#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|slices: triebits::Slices| {
    triebits::exercise_slices(slices);
});
