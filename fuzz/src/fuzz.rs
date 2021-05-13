#![no_main]
use blimpvec::*;
use dnstrie::bmpvec::*;
use libfuzzer_sys::fuzz_target;

mod blimpvec;

fuzz_target!(|data: &[u8]| {
    let mut bmp = BmpVec::new();
    let mut blimp = BlimpVec::new();

    for byte in data {
        let pos = byte & 63;
        match byte >> 6 {
            0 => {
                assert_eq!(bmp.len(), blimp.len());
                assert_eq!(bmp.contains(pos), blimp.contains(pos));
            }
            1 => assert_eq!(bmp.insert(pos, pos), blimp.insert(pos, pos)),
            2 => assert_eq!(bmp.remove(pos), blimp.remove(pos)),
            3 => assert_eq!(bmp.get(pos), blimp.get(pos)),
            _ => panic!("inconcievable!"),
        }
    }
});
