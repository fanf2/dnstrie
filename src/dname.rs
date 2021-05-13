const SHIFT_BRANCH: u8 = 0;
const SHIFT_NOBYTE: u8 = 1;
const SHIFT_BITMAP: u8 = 2;
const SHIFT_OFFSET: u8 = 48;

/// Generate the table that maps bytes in DNS names to bit positions.
///
/// The bit positions have to be between SHIFT_BITMAP and SHIFT_OFFSET.
/// Escaped byte ranges mostly fit in this space, except for those
/// above 'z', so when we reach the max we roll over to the next escape
/// character. After filling the table we ensure that the bit positions
/// for hostname characters and escape characters all fit.
///
/// A few non-hostname characters (between '-' and the digits, and
/// between '_' and lower case letters) are treated the same way as
/// hostname characters so that the escaped ranges are simpler.

// I can't write a for loop in a const fn, but I can use tail
// recursion, to a limited extent. I'm using two nested tail-recursive
// loops to limit the stack depth to 32, to avoid hitting compile-time
// evaluation limits.

struct TableGen {
    table: [u16; 256],
    bit_one: u8,
    bit_two: u8,
    escaping: bool,
}

const fn gen_byte_to_bits() -> [u16; 256] {
    let gen = TableGen {
        table: [0u16; 256],
        bit_one: SHIFT_BITMAP,
        bit_two: SHIFT_BITMAP,
        escaping: true,
    };
    for_byte_to_bits_hi(gen, 0)
}

const fn for_byte_to_bits_hi(gen: TableGen, hi: u8) -> [u16; 256] {
    let gen = for_byte_to_bits_lo(gen, hi, 0);
    if hi < 0xF0 {
        for_byte_to_bits_hi(gen, hi + 0x10)
    } else {
        gen.table
    }
}

const fn for_byte_to_bits_lo(mut gen: TableGen, hi: u8, lo: u8) -> TableGen {
    let byte = hi | lo;
    let i = byte as usize;
    match byte {
        // common characters
        b'-'..=b'9' | b'_'..=b'z' => {
            gen.escaping = false;
            gen.bit_one += 1;
            gen.table[i] = gen.bit_one as u16;
        }
        // map upper case to lower case
        b'A'..=b'Z' => {
            gen.table[i] = (
                (gen.bit_one + 1) + // bump past escape character
                    (b'a' - b'_') + // and skip non-letters
                    (byte - b'A'))  // count the alphabet
                as u16;
        }
        // non-hostname characters need to be escaped
        _ => {
            // do we need to bump to the next escape character?
            if !gen.escaping || gen.bit_two >= SHIFT_OFFSET {
                gen.escaping = true;
                gen.bit_one += 1;
                gen.bit_two = SHIFT_BITMAP;
            }
            gen.table[i] = (gen.bit_two as u16) << 8 | (gen.bit_one as u16);
            gen.bit_two += 1;
        }
    }
    if lo < 0x0F {
        for_byte_to_bits_lo(gen, hi, lo + 0x01)
    } else {
        gen
    }
}

pub const BYTE_TO_BITS: [u16; 256] = gen_byte_to_bits();
