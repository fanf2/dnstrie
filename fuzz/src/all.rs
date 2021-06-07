#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Action<'a> {
    BmpVec(&'a [u8]),
    DnsName(&'a [u8]),
    DnsText(&'a [u8]),
    TrieBits(&'a [u8]),
}

use Action::*;

fuzz_target!(|action: Action| {
    match action {
        BmpVec(bytes) => bmpvec::exercise_bytes(bytes),
        DnsName(bytes) => dnsname::exercise_wire(bytes),
        DnsText(bytes) => dnsname::exercise_text(bytes),
        TrieBits(bytes) => triebits::exercise_bytes(bytes),
    }
});
