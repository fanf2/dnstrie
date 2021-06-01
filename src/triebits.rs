//! Bitmap layout for DNS qp-trie
//! =============================

use crate::dnsname::*;
use crate::error::*;
use crate::scratchpad::*;

//const SHIFT_BRANCH: u8 = 0;
const SHIFT_NOBYTE: u8 = 1;
const SHIFT_BITMAP: u8 = 2;
const SHIFT_OFFSET: u8 = 48;

// a slight over-estimate
const MAX_TRIENAME: usize = MAX_NAME * 2 + 2;

/// A table that maps bytes in DNS names to bit positions, used by `trie_prep()`
pub const BYTE_TO_BITS: [(u8, u8); 256] = gen_byte_to_bits();

// 48*48 bytes is less than 2.5KB
pub const BITS_TO_BYTE: [[u8; 48]; 48] = gen_bits_to_byte();

#[derive(Debug, Default)]
pub struct TrieName {
    key: ScratchPad<u8, MAX_TRIENAME>,
}

impl TrieName {
    #[inline(always)]
    pub fn new() -> Self {
        TrieName { key: ScratchPad::new() }
    }

    pub fn as_slice(&self) {
        self.key.as_slice();
    }

    pub fn clear(&mut self) {
        self.key.clear();
    }

    pub fn from_dns_name<T>(&mut self, name: T) -> Result<()>
    where
        T: DnsLabels,
    {
        self.clear();
        // Skip the root label by starting at label 1
        for lab in 1..name.labs() {
            for &c in name.rlabel(lab).unwrap().iter() {
                let (one, two) = BYTE_TO_BITS[c as usize];
                self.key.push(one)?;
                if two > 0 {
                    self.key.push(two)?;
                }
            }
            self.key.push(SHIFT_NOBYTE)?;
        }
        // terminator is a double NOBYTE
        self.key.push(SHIFT_NOBYTE)
    }
}

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

const fn gen_byte_to_bits() -> [(u8, u8); 256] {
    let mut bit_one = SHIFT_BITMAP;
    let mut bit_two = SHIFT_BITMAP;
    let mut escaping = true;
    let mut table = [(0u8, 0u8); 256];
    let mut byte = 0;
    loop {
        let i = byte as usize;
        match byte {
            // common characters
            b'-'..=b'9' | b'_'..=b'z' => {
                escaping = false;
                bit_one += 1;
                table[i] = (bit_one, 0);
            }
            // map upper case to lower case
            b'A'..=b'Z' => {
                table[i] = (
                    (bit_one + 1) + // bump past escape character
                     (b'a' - b'_') + // and skip non-letters
                        (byte - b'A'), // count the alphabet
                    0,
                );
            }
            // non-hostname characters need to be escaped
            _ => {
                if !escaping || bit_two >= SHIFT_OFFSET {
                    // bump to the next escape character
                    escaping = true;
                    bit_one += 1;
                    bit_two = SHIFT_BITMAP;
                }
                table[i] = (bit_one, bit_two);
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

const fn gen_bits_to_byte() -> [[u8; 48]; 48] {
    let mut table = [[0u8; 48]; 48];
    let mut byte = 0;
    loop {
        if byte == b'A' {
            byte += 26;
        }
        let (one, two) = BYTE_TO_BITS[byte as usize];
        table[one as usize][two as usize] = byte;
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
    fn byte_to_bits() {
        for byte in b'a'..=b'z' {
            let lower = byte as usize;
            let upper = lower - 32;
            assert_eq!(BYTE_TO_BITS[upper], BYTE_TO_BITS[lower]);
        }
        for i in 0..=255 {
            let (one, two) = BYTE_TO_BITS[i];
            assert!(one >= SHIFT_BITMAP);
            assert!(one < SHIFT_OFFSET);
            assert!(two >= SHIFT_BITMAP || two == 0);
            assert!(two < SHIFT_OFFSET);
        }
        for i in 0..=254 {
            let j = i + 1;
            let (ilo, ihi) = BYTE_TO_BITS[i];
            let (jlo, jhi) = BYTE_TO_BITS[j];
            assert!(ilo <= jlo || i as u8 == b'Z');
            if ilo == jlo {
                assert!(ihi < jhi);
            }
        }
    }
}
