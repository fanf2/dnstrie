#![no_main]
use libfuzzer_sys::fuzz_target;
use dnstrie::test::prelude::*;

fuzz_target!(|wire: &[u8]| {
    dnsname::exercise_bytes(wire);
});
