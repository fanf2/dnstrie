#![no_main]
use libfuzzer_sys::fuzz_target;

mod blimpvec;

fuzz_target!(|data: &[u8]| { unimplemented!() });
