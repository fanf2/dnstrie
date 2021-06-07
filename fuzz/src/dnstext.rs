#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &[u8]| {
    dnsname::exercise_text(text);
});
