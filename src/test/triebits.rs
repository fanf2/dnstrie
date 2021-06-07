use crate::test::prelude::*;

#[derive(Arbitrary, Clone, Debug)]
pub struct Slices<'a> {
    one: &'a [u8],
    two: &'a [u8],
}

pub fn exercise_slices(slices: Slices) {
    let mut scratch1 = ScratchName::new();
    let mut scratch2 = ScratchName::new();
    if let Err(_) = scratch1.from_wire(slices.one, 0) {
        return;
    }
    if let Err(_) = scratch2.from_wire(slices.two, 0) {
        return;
    }
    let scratch_ord = scratch1.cmp(&scratch2);
    let mut triename1 = TrieName::new();
    let mut triename2 = TrieName::new();
    triename1.from_dns_name(&scratch1).unwrap();
    triename2.from_dns_name(&scratch2).unwrap();
    let slice1 = triename1.as_slice();
    let slice2 = triename2.as_slice();
    let slice_ord = slice1.cmp(slice2);
    assert_eq!(scratch_ord, slice_ord);
    let heap1 = triename1.make_dns_name().unwrap();
    let heap2 = triename2.make_dns_name().unwrap();
    assert_eq!(heap1, scratch1);
    assert_eq!(heap2, scratch2);
    let heap_ord = heap1.cmp(&heap2);
    assert_eq!(slice_ord, heap_ord);
}

pub fn exercise_bytes(bytes: &[u8]) {
    let slices = Unstructured::new(bytes).arbitrary().unwrap();
    exercise_slices(slices);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let mut rand = [0u8; 1000];
        for _ in 0..100 {
            rand.fill_with(|| fastrand::u8(..));
            exercise_bytes(&rand[..]);
        }
    }
}
