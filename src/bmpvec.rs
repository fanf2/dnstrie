use bmp::*;
use std::convert::TryInto;

pub struct BmpVec<T> {
    bmp: Bmp,
    ptr: *mut T,
}

impl<T> Drop for BmpVec<T> {
    fn drop(&mut self) {
        let _ = self.as_cooked_parts();
    }
}

impl<T> BmpVec<T> {
    pub fn new() -> BmpVec<T> {
        BmpVec::from_cooked_parts(Bmp::new(), Vec::new())
    }

    fn from_cooked_parts(bmp: Bmp, vec: Vec<T>) -> BmpVec<T> {
        assert_eq!(usize::from(bmp), vec.len());
        // ensure there is no excess capacity
        // because we don't have space to remember it
        let shrunk = vec.into_boxed_slice();
        // get a raw pointer to the slice as *mut [T]
        let slice = Box::into_raw(shrunk);
        // coerce to the element type for use by pointer::add()
        let ptr = slice as *mut T;
        BmpVec { bmp, ptr }
    }

    fn as_cooked_parts(&mut self) -> (Bmp, Vec<T>) {
        let len = self.bmp.into();
        let vec = unsafe { Vec::from_raw_parts(self.ptr, len, len) };
        (self.bmp, vec)
    }

    pub unsafe fn from_raw_parts(bmp: u64, ptr: *mut T) -> BmpVec<T> {
        BmpVec { bmp: Bmp::from_raw_parts(bmp), ptr }
    }

    pub unsafe fn into_raw_parts(self) -> (u64, *mut T) {
        (self.bmp.into_raw_parts(), self.ptr)
    }

    unsafe fn mask_ptr(&self, mask: Mask) -> *mut T {
        self.ptr.add(self.bmp & mask)
    }

    unsafe fn get_ptr<N>(&self, pos: N) -> Option<*mut T>
    where
        N: TryInto<u8>,
    {
        bitmask(pos)
            .filter(|&(bit, _)| self.bmp & bit)
            .map(|(_, mask)| self.mask_ptr(mask))
    }

    pub fn get<N>(&self, pos: N) -> Option<&T>
    where
        N: TryInto<u8>,
    {
        unsafe { self.get_ptr(pos).and_then(|ptr| ptr.as_ref()) }
    }

    pub fn get_mut<N>(&mut self, pos: N) -> Option<&mut T>
    where
        N: TryInto<u8>,
    {
        unsafe { self.get_ptr(pos).and_then(|ptr| ptr.as_mut()) }
    }

    pub fn contains<N>(&self, pos: N) -> bool
    where
        N: TryInto<u8>,
    {
        bitmask(pos).map_or(false, |(bit, _)| self.bmp & bit)
    }

    pub fn is_empty(&self) -> bool {
        self.bmp.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bmp.into()
    }

    pub fn insert<N>(&mut self, pos: N, val: T) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        self.set(pos, Some(val))
    }

    pub fn remove<N>(&mut self, pos: N) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        self.set(pos, None)
    }

    pub fn set<N>(&mut self, pos: N, val: Option<T>) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        match (bitmask(pos), val) {
            (Some((bit, mask)), Some(val)) if self.bmp & bit => unsafe {
                self.mask_ptr(mask)
                    .as_mut()
                    .map(|entry| std::mem::replace(entry, val))
            },
            (Some((bit, mask)), Some(val)) => {
                self.with_cooked_parts(bit, mask, val, |vec, rank, val| {
                    // try to avoid growing too much then immediately shrinking
                    vec.reserve(1);
                    vec.insert(rank, val);
                    None
                })
            }
            #[rustfmt::skip] // bananas
            (Some((bit, mask)), None) if self.bmp & bit => {
                self.with_cooked_parts(bit, mask, (), |vec, rank, _| {
                    Some(vec.remove(rank))
                })
            },
            (None, Some(_)) => {
                panic!("BmpVec position {:?} out of range", pos)
            }
            _ => None,
        }
    }

    fn with_cooked_parts<F, A>(
        &mut self,
        bit: Bit,
        mask: Mask,
        arg: A,
        mutate: F,
    ) -> Option<T>
    where
        F: Fn(&mut Vec<T>, usize, A) -> Option<T>,
    {
        let (mut bmp, mut vec) = self.as_cooked_parts();
        let ret = mutate(&mut vec, bmp & mask, arg);
        bmp ^= bit;
        std::mem::forget(std::mem::replace(
            self,
            BmpVec::from_cooked_parts(bmp, vec),
        ));
        ret
    }
}

/// 64-bit bitmaps
/// ==============

mod bmp {
    use std::convert::TryInto;

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
}
