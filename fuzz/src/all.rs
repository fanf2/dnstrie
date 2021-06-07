#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Action<'a> {
    DnsName(&'a [u8]),
    DnsText(&'a [u8]),
    BmpVec(&'a [u8]),
}

use Action::*;

fuzz_target!(|action: Action| {
    match action {
        DnsName(bytes) => dnsname::exercise_wire(bytes),
        DnsText(bytes) => dnsname::exercise_text(bytes),
        BmpVec(bytes) => bmpvec::exercise_bytes(bytes),
    }
});
