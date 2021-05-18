//! Bitmap layout for DNS qp-trie
//! =============================

// const SHIFT_BRANCH: u8 = 0;
// const SHIFT_NOBYTE: u8 = 1;
const SHIFT_BITMAP: u8 = 2;
const SHIFT_OFFSET: u8 = 48;

/// A table that maps bytes in DNS names to bit positions, used by `trie_prep()`

pub const BYTE_TO_BITS: [u16; 256] = gen_byte_to_bits();

/// Generate the table that maps bytes in DNS names to bit positions.
///
/// The bit positions have to be between SHIFT_BITMAP and SHIFT_OFFSET.
/// Escaped byte ranges mostly fit in this space, except for those
/// above 'z', so when we reach the max we roll over to the next escape
/// character.
///
/// A few non-hostname characters (between '-' and the digits, and
/// between '_' and lower case letters) are treated the same way as
/// hostname characters so that the escaped ranges are simpler.

const fn gen_byte_to_bits() -> [u16; 256] {
    let mut bit_one = SHIFT_BITMAP;
    let mut bit_two = SHIFT_BITMAP;
    let mut escaping = true;
    let mut table = [0u16; 256];
    let mut byte = 0;
    loop {
        let i = byte as usize;
        match byte {
            // common characters
            b'-'..=b'9' | b'_'..=b'z' => {
                escaping = false;
                bit_one += 1;
                table[i] = bit_one as u16;
            }
            // map upper case to lower case
            b'A'..=b'Z' => {
                table[i] = (
                    (bit_one + 1) + // bump past escape character
                     (b'a' - b'_') + // and skip non-letters
                     (byte - b'A')) // count the alphabet
                    as u16;
            }
            // non-hostname characters need to be escaped
            _ => {
                if !escaping || bit_two >= SHIFT_OFFSET {
                    // bump to the next escape character
                    escaping = true;
                    bit_one += 1;
                    bit_two = SHIFT_BITMAP;
                }
                table[i] = (bit_two as u16) << 8 | (bit_one as u16);
                bit_two += 1;
            }
        }
        if byte == 255 {
            return table;
        } else {
            byte += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn byte_to_bits() {
        for byte in b'a'..=b'z' {
            let lower = byte as usize;
            let upper = lower - 32;
            assert_eq!(BYTE_TO_BITS[upper], BYTE_TO_BITS[lower]);
        }
        for i in 0..=255 {
            let lo = (BYTE_TO_BITS[i] & 0xFF) as u8;
            let hi = (BYTE_TO_BITS[i] >> 8) as u8;
            assert!(lo >= SHIFT_BITMAP);
            assert!(lo < SHIFT_OFFSET);
            assert!(hi >= SHIFT_BITMAP);
            assert!(hi < SHIFT_OFFSET);
        }
        for i in 0..=254 {
            let j = i + 1;
            let ilo = (BYTE_TO_BITS[i] & 0xFF) as u8;
            let ihi = (BYTE_TO_BITS[i] >> 8) as u8;
            let jlo = (BYTE_TO_BITS[j] & 0xFF) as u8;
            let jhi = (BYTE_TO_BITS[j] >> 8) as u8;
            assert!(ilo <= jlo);
            if ilo == jlo {
                assert!(ihi < jhi);
            }
        }
    }
}
