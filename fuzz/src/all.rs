#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Fuzz<'a> {
    FuzzBmpVec(&'a [u8]),
    FuzzDnsName(&'a [u8]),
    FuzzDnsText(&'a [u8]),
    FuzzTrieBits(&'a [u8]),
}

use Fuzz::*;

fuzz_target!(|action: Fuzz| {
    match action {
        FuzzBmpVec(bytes) => bmpvec::exercise_bytes(bytes),
        FuzzDnsName(bytes) => dnsname::exercise_wire(bytes),
        FuzzDnsText(bytes) => dnsname::exercise_text(bytes),
        FuzzTrieBits(bytes) => triebits::exercise_bytes(bytes),
    }
});
