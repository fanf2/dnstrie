use crate::bmp::*;
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
        // get a raw pointer to the slice
        let slice = Box::into_raw(shrunk);
        // coerce to the element type for use by pointer::offset()
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
        Bmp::bitmask(pos)
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
        Bmp::bitmask(pos).map_or(false, |(bit, _)| self.bmp & bit)
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
        match Bmp::bitmask(pos) {
            Some((bit, mask)) if self.bmp & bit => unsafe {
                self.mask_ptr(mask)
                    .as_mut()
                    .map(|elem| std::mem::replace(elem, val))
            },
            Some((bit, mask)) => {
                let (mut bmp, mut vec) = self.as_cooked_parts();
                let rank = bmp & mask;
                bmp ^= bit;
                // try to avoid growing too much then immediately shrinking
                vec.reserve(1);
                vec.insert(rank, val);
                *self = BmpVec::from_cooked_parts(bmp, vec);
                None
            }
            None => panic!("BmpVec position {:?} out of range", pos),
        }
    }

    pub fn remove<N>(&mut self, pos: N) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        match Bmp::bitmask(pos) {
            Some((bit, mask)) if self.bmp & bit => {
                let (mut bmp, mut vec) = self.as_cooked_parts();
                let rank = bmp & mask;
                bmp ^= bit;
                let old = vec.remove(rank);
                *self = BmpVec::from_cooked_parts(bmp, vec);
                Some(old)
            }
            _ => None,
        }
    }
}
