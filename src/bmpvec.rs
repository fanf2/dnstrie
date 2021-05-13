//! Short bitmap-packed vectors
//! ===========================
//!
//! A [`BmpVec`] is a sparse vector of up to 64 elements. It only stores the
//! elements that are present, not the gaps between them. A bitmap ("bmp" for
//! short) indicates which elements are present or not.
//!
//! There is a trick to find an element in the underlying vector: create a
//! bitmask covering the bits in the bitmap less than the element we are
//! looking for, then count how many bits covered by the mask are set. This
//! can be done quickly with the `popcount` instruction, aka "population
//! count" or "Hamming weight", or (in Rust) `count_ones`, or "sideways add"
//! according to Knuth.
//!
//! See also "Hacker's Delight" by Henry S. Warren Jr, section 5-1.

use bmp::*;
use std::convert::TryInto;

/// A [`BmpVec`] is a sparse vector of up to 64 elements.
///
/// The elements are numbered between 0 and 63.
///
/// The `BmpVec` only stores the elements that are present, not the gaps
/// between them. Unlike a [`Vec`], there is no extra capacity. The storage
/// is always reallocated when an element is inserted or removed, so
/// compactness is prioritized more than speed of mutation.
///
/// A `BmpVec` is represented as two words: a bitmap indicating which
/// elements are present, and a pointer to the memory containing the
/// elements.
///
pub struct BmpVec<T> {
    bmp: Bmp,
    ptr: *mut T,
}

impl<T> Drop for BmpVec<T> {
    fn drop(&mut self) {
        let _ = unsafe { self.as_cooked_parts() };
    }
}

impl<T> Default for BmpVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BmpVec<T> {
    /// Constructs a new, empty `BmpVec`.
    pub fn new() -> BmpVec<T> {
        BmpVec::from_cooked_parts(Bmp::new(), Vec::new())
    }

    /// Returns `true` if there are no elementss in the `BmpVec`
    ///
    pub fn is_empty(&self) -> bool {
        self.bmp.is_empty()
    }

    /// Returns the number of elementss in the `BmpVec`
    ///
    pub fn len(&self) -> usize {
        self.bmp.into()
    }

    /// Returns `true` if there is an element at the given `pos`ition.
    ///
    pub fn contains<N>(&self, pos: N) -> bool
    where
        N: TryInto<u8>,
    {
        bitmask(pos).map_or(false, |(bit, _)| self.bmp & bit)
    }

    /// Get a reference to an element in the `BmpVec`
    ///
    /// This returns `None` if there is no element at `pos`.
    ///
    pub fn get<N>(&self, pos: N) -> Option<&T>
    where
        N: TryInto<u8>,
    {
        unsafe { self.get_ptr(pos).and_then(|ptr| ptr.as_ref()) }
    }

    /// Get a mutable reference to an element in the `BmpVec`
    ///
    /// This returns `None` if there is no element at `pos`.
    ///
    pub fn get_mut<N>(&mut self, pos: N) -> Option<&mut T>
    where
        N: TryInto<u8>,
    {
        unsafe { self.get_ptr(pos).and_then(|ptr| ptr.as_mut()) }
    }

    /// Set the `val`ue of the element at the given `pos`ition.
    ///
    /// If there was previously no element at the given `pos`ition then
    /// `None` is returned.
    ///
    /// If there was an element, then it is replaced with the new value and
    /// the old value is returned.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is not between 0 and 63.
    ///
    pub fn insert<N>(&mut self, pos: N, val: T) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        self.set(pos, Some(val))
    }

    /// Remove the element at the given `pos`ition.
    ///
    /// Returns the value of the element if there was one, or `None` if
    /// there was not.
    ///
    pub fn remove<N>(&mut self, pos: N) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        self.set(pos, None)
    }

    /// Set or clear the `val`ue of the element at the given `pos`ition.
    ///
    /// The old value of the element (or `None`) is returned.
    ///
    /// # Panics
    ///
    /// Panics if you try to set a value when `pos` is not between 0 and 63.
    ///
    pub fn set<N>(&mut self, pos: N, val: Option<T>) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        match (bitmask(pos), val) {
            (Some((bit, mask)), Some(val)) if self.bmp & bit => unsafe {
                self.ptr_plus(mask)
                    .as_mut()
                    .map(|entry| std::mem::replace(entry, val))
            },
            (Some((bit, mask)), Some(val)) => {
                self.with_cooked_parts(val, |bmp, vec, val| {
                    // try to avoid growing too much then immediately shrinking
                    vec.reserve(1);
                    vec.insert(bmp & mask, val);
                    (bmp ^ bit, None)
                })
            }
            #[rustfmt::skip] // bananas
            (Some((bit, mask)), None) if self.bmp & bit => {
                self.with_cooked_parts((), |bmp, vec, _| {
                    let old = vec.remove(bmp & mask);
                    (bmp ^ bit, Some(old))
                })
            },
            (None, Some(_)) => {
                panic!("BmpVec position {:?} out of range", pos)
            }
            _ => None,
        }
    }

    /// Construct a `BmpVec` from a raw bitmap and pointer.
    ///
    /// This is the inverse of [`BmpVec::into_raw_parts()`]
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren’t
    /// checked, as for [`Vec::from_raw_parts()`].
    ///
    /// The number of bits set in `bmp` must be equal to both the length and
    /// capacity of the allocation at `ptr`.
    ///
    /// The ownership of ptr is transferred to the `BmpVec`.
    ///
    pub unsafe fn from_raw_parts(bmp: u64, ptr: *mut T) -> BmpVec<T> {
        BmpVec { bmp: Bmp::from_raw_parts(bmp), ptr }
    }

    /// Unwrap a `BmpVec` into a raw bitmap and pointer.
    ///
    /// This consumes the `BitVec`.
    ///
    /// # Safety
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the `BmpVec`. The only way to do this is to
    /// convert the raw parts back using [`BmpVec::from_raw_parts()`]
    ///
    pub unsafe fn into_raw_parts(self) -> (u64, *mut T) {
        (self.bmp.into_raw_parts(), self.ptr)
    }

    /// Expand a `BmpVec` into a pair of a bitmap and a vector.
    ///
    /// The vector is easily mutable, unlike the raw pointer inside the
    /// `BmpVec`.
    ///
    /// # Safety
    ///
    /// This does not consume the `BmpVec`, so there is danger of a double
    /// free if both the `BmpVec` and `Vec` are dropped. The caller
    /// must ensure that one or the other is forgotten, typically using
    /// [`BmpVec::replace_cooked_parts()`].
    ///
    unsafe fn as_cooked_parts(&mut self) -> (Bmp, Vec<T>) {
        let len = self.bmp.into();
        (self.bmp, Vec::from_raw_parts(self.ptr, len, len))
    }

    /// Construct a `BmpVec` from a pair of a bitmap and a vector.
    ///
    /// The vector is consumed.
    ///
    /// Reallocates the memory if there is any excess capacity.
    ///
    /// # Panics
    ///
    /// Panics if the number of bits set in the bitmap is not the same as the
    /// length of the vector.
    ///
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

    /// Replace a `BmpVec` with a new bitmap and vector.
    ///
    /// This is for cleaning up after calling [`BmpVec::as_cooked_parts()`]
    /// and mutating the results. The replacement is constructed using
    /// [`BmpVec::from_cooked_parts()`].
    ///
    /// # Safety
    ///
    /// This forgets the old contents of the `BmpVec` to prevent a double
    /// free. This can lead to a leak if
    /// [`BmpVec::replace_cooked_parts()`] is not used correctly.
    ///
    unsafe fn replace_cooked_parts(&mut self, bmp: Bmp, vec: Vec<T>) {
        std::mem::forget(std::mem::replace(
            self,
            BmpVec::from_cooked_parts(bmp, vec),
        ));
    }

    /// Mutate a `BmpVec` in relative safety.
    ///
    /// You provide a closure which is called with the bitmap and a vector.
    /// The closure can mutate the vector, then return a replacement bitmap
    /// and a return value. The closure has an extra argument so that you can
    /// pass through values with single ownership (without capturing it in
    /// the closure).
    ///
    /// This keeps the safety requirements of [`BmpVec::as_cooked_parts()`]
    /// and [`BmpVec::replace_cooked_parts()`].
    ///
    /// # Panics
    ///
    /// Panics if the number of bits set in the replacement bitmap is not the
    /// same as the length of the mutated vector.
    ///
    fn with_cooked_parts<F, A>(&mut self, arg: A, mutate: F) -> Option<T>
    where
        F: Fn(Bmp, &mut Vec<T>, A) -> (Bmp, Option<T>),
    {
        unsafe {
            let (bmp, mut vec) = self.as_cooked_parts();
            let (bmp, ret) = mutate(bmp, &mut vec, arg);
            self.replace_cooked_parts(bmp, vec);
            ret
        }
    }

    /// Get a raw pointer to an element in the `BmpVec`
    ///
    /// This returns `None` if there is no element at `pos`.
    ///
    /// # Safety
    ///
    /// When converting the raw pointer to a ref, the ref's ownership must be
    /// consistent with `self`'s ownership.
    ///
    unsafe fn get_ptr<N>(&self, pos: N) -> Option<*mut T>
    where
        N: TryInto<u8>,
    {
        bitmask(pos)
            .filter(|&(bit, _)| self.bmp & bit)
            .map(|(_, mask)| self.ptr_plus(mask))
    }

    /// Get a raw pointer to the `mask`'s element
    ///
    /// # Safety
    ///
    /// The element should be present for this to be meaningful, like the
    /// checks in [`BmpVec::get_ptr()`].
    ///
    unsafe fn ptr_plus(&self, mask: Mask) -> *mut T {
        self.ptr.add(self.bmp & mask)
    }
}

/// 64-bit bitmaps
/// ==============
///
/// These types are in a separate module with a limited interface to
/// make it harder to use the wrong value in the wrong context.
///
/// [`Bmp`] is the only type with methods.
///
/// There are a few operators with slightly weird but convenient behaviour:
///
///   * `usize::from(Bmp)` : count set bits
///
///   * `Bmp & Mask -> usize` : count set bits covered by the mask
///
///   * `Bmp & Bit -> bool` : test whether a `Bit` is present
///
/// The other operator is `Bmp ^ Bit` which is not weird.

mod bmp {
    use std::convert::TryInto;

    /// A bitmap to identify which elements are present in a
    /// [`BmpVec`][super::BmpVec]
    ///
    #[derive(Clone, Copy)]
    pub struct Bmp(u64);

    /// exactly one bit set to identify a particular element
    ///
    /// constructed by [`bitmask()`]
    ///
    #[derive(Clone, Copy)]
    pub struct Bit(u64);

    /// all the bits less than the accompanying [`Bit`]
    ///
    /// constructed by [`bitmask()`]
    ///
    #[derive(Clone, Copy)]
    pub struct Mask(u64);

    /// Get the [`Bit`] at a given `pos`ition, and a [`Mask`] covering the
    /// lesser bits. Both are `None` if the position is out of bounds.
    ///
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

    impl std::ops::BitAnd<Bit> for Bmp {
        type Output = bool;
        fn bitand(self, bit: Bit) -> bool {
            self.0 & bit.0 != 0
        }
    }

    impl std::ops::BitXor<Bit> for Bmp {
        type Output = Bmp;
        fn bitxor(self, bit: Bit) -> Bmp {
            Bmp(self.0 ^ bit.0)
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
        /// Create an empty bitmap
        pub const fn new() -> Bmp {
            Bmp(0)
        }

        /// Is the bitmap empty?
        pub fn is_empty(self) -> bool {
            self.0 == 0
        }

        /// Create a bitmap from some previously-obtained guts.
        ///
        /// # Safety
        ///
        /// This function is marked unsafe because it is only for use by the
        /// unsafe parts of [`BmpVec`][super::BmpVec]
        ///
        pub unsafe fn from_raw_parts(bmp: u64) -> Bmp {
            Bmp(bmp)
        }

        /// Get hold of the guts of the bitmap.
        ///
        /// # Safety
        ///
        /// This function is marked unsafe because it is only for use by the
        /// unsafe parts of [`BmpVec`][super::BmpVec]
        ///
        pub unsafe fn into_raw_parts(self) -> u64 {
            self.0
        }
    }
}
