use crate::test::prelude::*;

#[derive(Arbitrary, Copy, Clone, Debug)]
pub enum Action {
    Contains(u8),
    Insert(u8),
    Remove(u8),
    Get(u8),
    Len,
    IsEmpty,
    BmpIter,
    BlimpIter,
    Keys,
    Values,
    FromBlimp,
    FromBmp,
    Format,
    Clear,
}

use Action::*;

pub fn exercise(actions: &[Action]) {
    let mut bmp = BmpVec::new();
    let mut blimp = BlimpVec::new();
    for &action in actions {
        match action {
            Contains(pos) => {
                assert_eq!(bmp.contains(pos & 63), blimp.contains(pos & 63))
            }
            Insert(pos) => {
                assert_eq!(
                    bmp.insert(pos & 63, pos),
                    blimp.insert(pos & 63, pos)
                )
            }
            Remove(pos) => {
                assert_eq!(bmp.remove(pos & 63), blimp.remove(pos & 63))
            }
            Get(pos) => assert_eq!(bmp.get(pos & 63), blimp.get(pos & 63)),
            Len => assert_eq!(bmp.len(), blimp.len()),
            IsEmpty => assert_eq!(bmp.is_empty(), blimp.is_empty()),
            BmpIter => {
                for (pos, elem) in bmp.iter() {
                    assert_eq!(Some(elem), blimp.get(pos));
                }
            }
            BlimpIter => {
                for (pos, elem) in blimp.iter() {
                    assert_eq!(Some(elem), bmp.get(pos));
                }
            }
            Keys => {
                let bmp_keys: Vec<u8> = bmp.keys().collect();
                let blimp_keys: Vec<u8> = blimp.keys().collect();
                assert_eq!(bmp_keys, blimp_keys);
            }
            Values => {
                let bmp_values: Vec<u8> = bmp.values().copied().collect();
                let blimp_values: Vec<u8> = blimp.values().copied().collect();
                assert_eq!(bmp_values, blimp_values);
            }
            FromBlimp => {
                let from_blimp = BmpVec::from(&blimp);
                assert_eq!(bmp, from_blimp);
            }
            FromBmp => {
                let from_bmp = BlimpVec::from(&bmp);
                assert_eq!(from_bmp, blimp);
            }
            Format => {
                let bmptxt = format!("{:?}", &bmp);
                let blimptxt = format!("{:?}", &blimp);
                assert_eq!(&bmptxt[3..], &blimptxt[5..]);
            }
            Clear => {
                for pos in 0..=63 {
                    assert_eq!(bmp.remove(pos), blimp.remove(pos));
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::iter::repeat_with;

    #[test]
    fn test() -> Result<()> {
        type Actions = [Action; 100];
        let (min, max) = Actions::size_hint(0);
        let bytes = max.unwrap_or(min);
        let r: Vec<u8> = repeat_with(|| fastrand::u8(..)).take(bytes).collect();
        let a: Actions = Unstructured::new(&r[..]).arbitrary()?;
        exercise(&a[..]);
        Ok(())
    }
}
