//! Scratch space for various kinds of DNS name
//! ===========================================
//!
//! This is an append-only fixed-size memory area that avoids
//! initializing the elements before they are added.
//!
//! It can be cleared (reset to empty) and re-used. Rust prevents us
//! from muddling up the old and new contents of a `ScratchPad`, because
//! we can't clear it while we have a reference to a slice of its contents.
//!
//! If an append causes an overflow, [`Error::NameLength`] is returned,
//! which is suitable when parsing DNS names from the wire. This means
//! that a `ScratchPad` can be sized to exactly match the protocol
//! limits [`crate::dnsname::MAX_NAME`] and [`crate::dnsname::MAX_LABS`]
//! and there's no need for any overflow checking before writing to the
//! `ScratchPad`.

use crate::prelude::*;
use core::mem::MaybeUninit;

pub struct ScratchPad<T, const SIZE: usize> {
    uninit: [MaybeUninit<T>; SIZE],
    end: usize,
}

impl<T, const SIZE: usize> Default for ScratchPad<T, SIZE> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const SIZE: usize> std::fmt::Debug for ScratchPad<T, SIZE>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = std::any::type_name::<T>();
        f.debug_struct(&format!("ScratchPad<{}, {}>", t, SIZE))
            .field("as_slice", &self.as_slice())
            .finish()
    }
}

impl<T, const SIZE: usize> ScratchPad<T, SIZE> {
    /// Create a new empty scratch pad.
    #[inline(always)]
    pub fn new() -> Self {
        // SAFETY: `assume_init()` is safe because an array of
        // `MaybeUninit`s does not require initialization.
        ScratchPad {
            uninit: unsafe { MaybeUninit::uninit().assume_init() },
            end: 0,
        }
    }

    /// Reset the scratch pad to empty.
    pub fn clear(&mut self) {
        self.end = 0;
    }

    /// Is the scratch pad empty?
    pub fn is_empty(&self) -> bool {
        self.end == 0
    }

    /// The number of initialized elements in the scratch pad.
    pub fn len(&self) -> usize {
        self.end
    }

    /// Get a slice covering the initialized part of the scratch pad.
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: we have initialized everything up to end, and we can't
        // mutate it again while there's a shared borrow; the origin of the
        // pointer means its alignment, size, and nullity are OK. To persuade
        // Stacked Borrows that this is OK, we need to derive the pointer
        // from a borrow that covers the whole of the `uninit` array.
        let ptr = &self.uninit[..] as *const [MaybeUninit<T>] as *const T;
        unsafe { std::slice::from_raw_parts(ptr, self.end) }
    }

    #[inline(always)]
    fn get_mut(&mut self, pos: usize) -> Result<*mut T> {
        Ok(self.uninit.get_mut(pos).ok_or(ScratchOverflow)?.as_mut_ptr())
    }

    #[inline(always)]
    pub fn append(&mut self, elems: &[T]) -> Result<()> {
        let len = elems.len();
        let src = elems.as_ptr();
        let dst = self.get_mut(self.end)?;
        self.get_mut(self.end + len - 1)?;
        // SAFETY: bounds have been checked; the origins of the pointers
        // mean they don't overlap and alignment and nullity are OK.
        unsafe { dst.copy_from_nonoverlapping(src, len) };
        self.end += len;
        Ok(())
    }

    #[inline(always)]
    pub fn push(&mut self, elem: T) -> Result<()> {
        let ptr = self.get_mut(self.end)?;
        // SAFETY: the pointer is within bounds and properly aligned.
        unsafe { ptr.write(elem) };
        self.end += 1;
        Ok(())
    }
}
