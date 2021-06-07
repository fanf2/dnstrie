#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|wire: &[u8]| {
    dnsname::exercise_bytes(wire);
});
