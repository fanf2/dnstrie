//! 64-bit bitmaps
//! ==============
//!
//! The [`Bmp`][] type helps to avoid mixing up various word-sized
//! integer types.

use std::convert::TryInto;

#[derive(Clone, Copy)]
pub struct Bmp(u64);

#[derive(Clone, Copy)]
pub struct Bit(u64);

#[derive(Clone, Copy)]
pub struct Mask(u64);

impl std::ops::BitAnd<Bit> for Bmp {
    type Output = bool;
    fn bitand(self, bit: Bit) -> bool {
        self.0 & bit.0 != 0
    }
}

impl std::ops::BitXorAssign<Bit> for Bmp {
    fn bitxor_assign(&mut self, bit: Bit) {
        self.0 ^= bit.0;
    }
}

impl std::ops::BitAnd<Mask> for Bmp {
    type Output = usize;
    fn bitand(self, mask: Mask) -> usize {
        (self.0 & mask.0).count_ones() as usize
    }
}

impl From<Bmp> for usize {
    fn from(bmp: Bmp) -> usize {
        bmp.0.count_ones() as usize
    }
}

impl Bmp {
    /// create an empty bitmap
    pub const fn new() -> Bmp {
        Bmp(0)
    }

    /// Get the bit at a given `pos`ition, and a mask covering the
    /// lesser bits. Both are `None` if the position is out of bounds.
    pub fn bitmask<N>(pos: N) -> Option<(Bit, Mask)>
    where
        N: TryInto<u8>,
    {
        match pos.try_into() {
            Ok(shift @ 0..=63) => {
                let bit = 1u64 << shift;
                Some((Bit(bit), Mask(bit - 1)))
            }
            _ => None,
        }
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// create a bitmap from some previously-obtained guts
    pub unsafe fn from_raw_parts(bmp: u64) -> Bmp {
        Bmp(bmp)
    }

    /// get hold of the guts of the bitmap
    pub unsafe fn into_raw_parts(self) -> u64 {
        self.0
    }
}
