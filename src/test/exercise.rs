use crate::bmpvec::*;
use crate::test::blimpvec::*;

pub fn bmpvec_blimpvec(data: &[u8]) {
    let mut bmp = BmpVec::new();
    let mut blimp = BlimpVec::new();

    for byte in data {
        let pos = byte & 63;
        match byte >> 6 {
            0 => {
                assert_eq!(bmp.insert(pos, pos), blimp.insert(pos, pos));
            }
            1 => {
                assert_eq!(bmp.remove(pos), blimp.remove(pos));
            }
            2 => {
                assert_eq!(bmp.get(pos), blimp.get(pos));
            }
            _ => match byte % 11 {
                0 => assert_eq!(bmp.len(), blimp.len()),
                1 => assert_eq!(bmp.is_empty(), blimp.is_empty()),
                2 => assert_eq!(bmp.contains(pos), blimp.contains(pos)),
                3 => {
                    for (pos, elem) in bmp.iter() {
                        assert_eq!(Some(elem), blimp.get(pos));
                    }
                }
                4 => {
                    for (pos, elem) in blimp.iter() {
                        assert_eq!(Some(elem), bmp.get(pos));
                    }
                }
                5 => {
                    let bmp_keys: Vec<u8> = bmp.keys().collect();
                    let blimp_keys: Vec<u8> = blimp.keys().collect();
                    assert_eq!(bmp_keys, blimp_keys);
                }
                6 => {
                    let bmp_values: Vec<u8> = bmp.values().copied().collect();
                    let blimp_values: Vec<u8> =
                        blimp.values().copied().collect();
                    assert_eq!(bmp_values, blimp_values);
                }
                7 => {
                    let from_blimp = BmpVec::from(&blimp);
                    assert_eq!(bmp, from_blimp);
                }
                8 => {
                    let from_bmp = BlimpVec::from(&bmp);
                    assert_eq!(from_bmp, blimp);
                }
                9 => {
                    let bmptxt = format!("{:?}", &bmp);
                    let blimptxt = format!("{:?}", &blimp);
                    assert_eq!(&bmptxt[3..], &blimptxt[5..]);
                }
                10 => {
                    for pos in 0..=63 {
                        assert_eq!(bmp.remove(pos), blimp.remove(pos));
                    }
                }
                _ => panic!("inconcievable!"),
            },
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
