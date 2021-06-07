use crate::test::prelude::*;

pub fn exercise_bytes(wire: &[u8]) {
    if wire.len() < 1 {
        return;
    }
    let start = wire[0] as usize;
    let mut wire_labels = WireLabels::<u16>::new();
    if let Err(_) = wire_labels.from_wire(wire, start) {
        return;
    }
    let mut scratch_name = ScratchName::new();
    if let Err(err) = scratch_name.from_wire(wire, start) {
        eprintln!("wire_labels {}", wire_labels);
        eprintln!("wire_labels.nlen {}", wire_labels.nlen());
        panic!("unexpected error {:#?}", err);
    }
    assert_eq!(wire_labels, scratch_name);
    assert!(wire_labels <= scratch_name);
    assert!(wire_labels >= scratch_name);
    let heap1 = HeapName::from(&wire_labels);
    let heap2 = HeapName::from(&scratch_name);
    assert_eq!(heap1, heap2);
    assert!(heap1 <= heap2);
    assert!(heap1 >= heap2);
    assert!(wire_labels <= heap2);
    assert!(wire_labels >= heap2);
    assert!(heap1 <= scratch_name);
    assert!(heap1 >= scratch_name);
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
