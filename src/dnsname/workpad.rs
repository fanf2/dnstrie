//! Working space for various kinds of DNS name
//! ===========================================
//!
//! This is an append-only fixed-size memory area that avoids
//! initializing the elements before they are added.

pub struct WorkPad<T, const SIZE: usize> {
    uninit: [std::mem::MaybeUninit<T>; SIZE],
    end: usize,
}

impl<T, const SIZE: usize> Default for WorkPad<T, SIZE> {
    fn default() -> Self {
        // SAFETY: The `assume_init` is safe because an array of
        // `MaybeUninit`s does not require initialization.
        WorkPad {
            uninit: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
            end: 0,
        }
    }
}

impl<T, const SIZE: usize> std::fmt::Debug for WorkPad<T, SIZE>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({:?})", std::any::type_name::<Self>(), self.as_slice())
    }
}

impl<T, const SIZE: usize> WorkPad<T, SIZE> {
    #[inline(always)]
    pub fn new() -> Self {
        Default::default()
    }

    pub fn len(&self) -> usize {
        self.end
    }

    pub fn is_empty(&self) -> bool {
        self.end == 0
    }

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: we have initialized everything up to end, and we won't
        // mutate it again; the origin of the pointer means its alignment,
        // size, and nullity are OK.
        let ptr = self.uninit[0].as_ptr();
        unsafe { std::slice::from_raw_parts(ptr, self.end) }
    }

    pub fn append(&mut self, elems: &[T]) {
        let dst = self.uninit[self.end].as_mut_ptr();
        let src = elems.as_ptr();
        let len = elems.len();
        assert!(self.end + len <= SIZE);
        // SAFETY: bounds have been checked; the origins of the pointers
        // mean they don't overlap and alignment and nullity are OK.
        unsafe { dst.copy_from_nonoverlapping(src, len) };
        self.end += len;
    }

    pub fn push(&mut self, elem: T) {
        let ptr = self.uninit[self.end].as_mut_ptr();
        // SAFETY: the pointer is within bounds and properly aligned.
        unsafe { ptr.write(elem) };
        self.end += 1;
    }
}
