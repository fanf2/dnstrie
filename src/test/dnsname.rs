use crate::test::prelude::*;

pub fn exercise_wire(wire: &[u8]) {
    if wire.len() < 1 {
        return;
    }
    let start = wire[0] as usize;
    let mut wire_labels = WireLabels::<u16>::new();
    if let Err(_) = wire_labels.from_wire(wire, start) {
        return;
    }
    let mut scratch1 = ScratchName::new();
    if let Err(err) = scratch1.from_wire(wire, start) {
        eprintln!("wire_labels {}", wire_labels);
        eprintln!("wire_labels.nlen {}", wire_labels.nlen());
        panic!("unexpected error {:#?}", err);
    }
    assert_eq!(wire_labels, scratch1);
    assert!(wire_labels <= scratch1);
    assert!(wire_labels >= scratch1);
    let heap1 = HeapName::from(&wire_labels);
    let heap2 = HeapName::from(&scratch1);
    assert_eq!(heap1, heap2);
    assert!(heap1 <= heap2);
    assert!(heap1 >= heap2);
    assert!(wire_labels <= heap2);
    assert!(wire_labels >= heap2);
    assert!(heap1 <= scratch1);
    assert!(heap1 >= scratch1);
    let mut text1 = format!("{}", wire_labels);
    let text2 = format!("{}", scratch1);
    // SAFETY: make_ascii_lowercase() does not affect UTF-8 validity
    unsafe { text1.as_bytes_mut().make_ascii_lowercase() };
    assert_eq!(text1, text2);
    let mut scratch2 = ScratchName::new();
    scratch2.from_text(text1.as_bytes()).unwrap();
    assert_eq!(scratch1, scratch2);
    assert!(scratch1 <= scratch2);
    assert!(scratch1 >= scratch2);
    let heap3 = HeapName::try_from(text2.as_str()).unwrap();
    assert_eq!(scratch2, heap3);
    assert_eq!(wire_labels, heap3);
    let text3 = format!("{}", heap3);
    assert_eq!(text2, text3);
    let heap4 = HeapName::try_from(scratch1.name()).unwrap();
    assert_eq!(scratch1, heap4);
    assert_eq!(heap3, heap4);
}

pub fn exercise_text(text: &[u8]) {
    let utf8 = match std::str::from_utf8(text) {
        Ok(ok) => ok,
        Err(_) => return,
    };
    let mut scratch = ScratchName::new();
    let len = match scratch.from_text(text) {
        Ok(ok) => ok,
        Err(_) => return,
    };
    let heap = match HeapName::try_from(&utf8[0..len]) {
        Ok(ok) => ok,
        Err(err) => panic!("unexpected error {:#?}", err),
    };
    assert_eq!(scratch, heap);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_wire() {
        let mut rand = [0u8; 1000];
        for _ in 0..100 {
            rand.fill_with(|| fastrand::u8(..));
            exercise_wire(&rand[..]);
        }
    }

    #[test]
    fn test_text() {
        let mut rand = [0u8; 1000];
        for _ in 0..100 {
            rand.fill_with(|| fastrand::u8(..));
            exercise_text(&rand[..]);
        }
    }
}
