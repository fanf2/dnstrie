#![no_main]
use dnstrie::test::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|actions: Vec<bmpvec::Action>| {
    bmpvec::exercise_actions(&actions[..]);
});
