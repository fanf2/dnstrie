#![no_main]
use dnstrie::test::exercise::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    bmpvec_blimpvec(data);
});
