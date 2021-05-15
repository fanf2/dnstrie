use crate::bmpvec::*;
use crate::test::blimpvec::*;

pub fn bmpvec_blimpvec(data: &[u8]) {
    let mut bmp = BmpVec::new();
    let mut blimp = BlimpVec::new();

    for byte in data {
        eprintln!("{:x}", byte);
        let pos = byte & 63;
        match byte >> 6 {
            0 => {
                // thorough cromulence check
                eprintln!("len");
                assert_eq!(bmp.len(), blimp.len());
                eprintln!("empty");
                assert_eq!(bmp.is_empty(), blimp.is_empty());
                eprintln!("contains");
                assert_eq!(bmp.contains(pos), blimp.contains(pos));
                eprintln!("bmp iter");
                for (pos, elem) in bmp.iter() {
                    assert_eq!(Some(elem), blimp.get(pos));
                }
                eprintln!("blimp iter");
                for (pos, elem) in blimp.iter() {
                    assert_eq!(Some(elem), bmp.get(pos));
                }
                eprintln!("keys");
                let bmp_keys: Vec<u8> = bmp.keys().collect();
                let blimp_keys: Vec<u8> = blimp.keys().collect();
                assert_eq!(bmp_keys, blimp_keys);
                eprintln!("values");
                let bmp_values: Vec<u8> = bmp.values().copied().collect();
                let blimp_values: Vec<u8> = blimp.values().copied().collect();
                assert_eq!(bmp_values, blimp_values);
            }
            1 => {
                eprintln!("bmp insert");
                let left = bmp.insert(pos, pos);
                eprintln!("blimp insert");
                let right = blimp.insert(pos, pos);
                assert_eq!(left, right);
            }
            2 => {
                eprintln!("remove");
                assert_eq!(bmp.remove(pos), blimp.remove(pos));
            }
            3 => {
                eprintln!("get");
                assert_eq!(bmp.get(pos), blimp.get(pos));
            }
            _ => panic!("inconcievable!"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::iter::repeat_with;

    #[test]
    fn test() {
        eprintln!("rand");
        let v: Vec<u8> = repeat_with(|| fastrand::u8(..)).take(1000).collect();
        eprintln!("exercise");
        bmpvec_blimpvec(&v[..]);
    }
}
