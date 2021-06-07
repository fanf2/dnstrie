#![no_main]
use libfuzzer_sys::fuzz_target;
use dnstrie::test::prelude::*;

fuzz_target!(|actions: Vec<bmpvec::Action>| {
    bmpvec::exercise_actions(&actions[..]);
});
