use crate::test::prelude::*;

#[derive(Arbitrary, Clone, Debug)]
pub struct Vecs {
    one: Vec<u8>,
    two: Vec<u8>,
}

pub fn exercise_vecs(vecs: Vecs) {
    let mut scratch1 = ScratchName::new();
    let mut scratch2 = ScratchName::new();
    if let Err(_) = scratch1.from_wire(&vecs.one[..], 0) {
        return;
    }
    if let Err(_) = scratch2.from_wire(&vecs.two[..], 0) {
        return;
    }
    let scratch_ord = scratch1.cmp(&scratch2);
    let mut triename1 = TrieName::new();
    let mut triename2 = TrieName::new();
    triename1.from_dns_name(&scratch1);
    triename2.from_dns_name(&scratch2);
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

// make a vec more likely to look like a wire format DNS name
fn nominate(v: &mut Vec<u8>) {
    let mut pos = 0;
    let mut len = 0;
    while pos < MAX_NAME && pos < v.len() {
        len = 0x3F & v[pos] as usize;
        v[pos] = len as u8;
        pos += 1 + len;
        if len == 0 {
            return;
        }
    }
    if len > 0 {
        v[pos - len - 1] = 0;
    }
}

pub fn exercise_bytes(bytes: &[u8]) {
    let mut vecs: Vecs = Unstructured::new(bytes).arbitrary().unwrap();
    nominate(&mut vecs.one);
    nominate(&mut vecs.two);
    exercise_vecs(vecs);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let mut rand = [0u8; 1000];
        for _ in 0..1000 {
            rand.fill_with(|| fastrand::u8(..));
            exercise_bytes(&rand[..]);
        }
    }
}
