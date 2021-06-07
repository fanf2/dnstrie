#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|actions: Vec<dnstrie::test::bmpvec::Action>| {
    dnstrie::test::bmpvec::exercise(&actions[..]);
});
