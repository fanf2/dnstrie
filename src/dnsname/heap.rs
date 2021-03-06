//! A DNS name owned and allocated on the heap
//! ==========================================
//!
//! A `HeapName` is intended to be reasonably efficient:
//!
//!   * it includes an index of the label positions, so it doesn't
//!     need to be re-parsed;
//!
//!   * the label index and name share a single allocation, made in
//!     one shot without reallocation;
//!
//!   * it only uses a single word to refer to the allocation.

use crate::prelude::*;

/// A DNS name owned and allocated on the heap
///
/// # Layout
///
/// The allocation contains one byte for the label count, that many
/// bytes for the label positions, then the bytes of the name. Label
/// positions are counted from the start of the name (a byte is only
/// just big enough for that). The last label is the root zone, so its
/// position is the length of the name minus one.
///
/// The maximum heap allocation is [`MAX_NAME`] plus
/// [`MAX_LABS`] plus a byte for the label count, totalling
/// 384 bytes.
///
/// A `HeapName` is never empty.
///
/// # Safety
///
/// Many of [`HeapName`]'s methods include unsafe code that assumes
/// the layout is correct, for instance, the label posisions must be
/// within the name.
///
///   * the allocation size matches the `HeapLen`
///
///   * it is non-null, properly aligned, and fully initialized
///
/// This safety requirement is established by the constructor,
/// `impl From<ScratchName>`. After that point, the name is
/// immutable so it remains safe.
///
pub struct HeapName {
    mem: *const u8,
    // NOTE: the marker tells dropck that we logically own some bytes
    _marker: PhantomData<u8>,
}

impl HeapName {
    // SAFETY: callers must satisfy the requirements described at
    // [`HeapName`] under "Safety"
    unsafe fn from_vec(vec: Vec<u8>) -> Self {
        let shrunk = vec.into_boxed_slice();
        let slice_ptr = Box::into_raw(shrunk);
        HeapName::from_raw_parts(slice_ptr as *const u8)
    }

    /// # Safety
    /// The argument must previously have been returned by
    /// `HeapName::as_ptr()` or `HeapName::into_ptr()`
    pub unsafe fn from_raw_parts(mem: *const u8) -> Self {
        HeapName { mem, _marker: PhantomData }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.mem
    }

    /// # Safety
    /// The caller becomes responsible for the memory previously owned
    /// by the HeapName.
    pub unsafe fn into_ptr(self) -> *const u8 {
        let ptr = self.mem;
        std::mem::forget(self); // avoid double free
        ptr
    }
}

impl Drop for HeapName {
    fn drop(&mut self) {
        let ptr = self.mem as *mut u8;
        let len = self.heap_len();
        // SAFETY: see [`HeapName`] under "Safety"
        let _ = unsafe { Vec::from_raw_parts(ptr, len, len) };
    }
}

/// SAFETY: the data in a [`HeapName`] is unaliased.
unsafe impl Send for HeapName {}

/// SAFETY: the data in a [`HeapName`] is unaliased.
unsafe impl Sync for HeapName {}

impl_dns_name!(HeapName);

impl DnsLabels for HeapName {
    fn labs(&self) -> usize {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { self.mem.read() as usize }
    }

    fn nlen(&self) -> usize {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { self.mem.add(self.labs()).read() as usize + 1 }
    }

    fn label(&self, lab: usize) -> Option<&[u8]> {
        DnsName::label(self, lab)
    }
}

impl DnsName for HeapName {
    fn name(&self) -> &[u8] {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe {
            let name = self.mem.add(1 + self.labs());
            std::slice::from_raw_parts(name, self.nlen())
        }
    }

    fn lpos(&self) -> &[u8] {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe {
            let lpos = self.mem.add(1);
            std::slice::from_raw_parts(lpos, self.labs())
        }
    }
}

impl std::fmt::Debug for HeapName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("HeapName")
            .field("lpos", &self.lpos())
            .field("name", &self.name())
            .finish()
    }
}

/// Calculate the allocation size for a [`HeapName`],
///
/// This is just a small extension to the [`DnsName`] trait,
/// specific to the needs of a [`HeapName`].
///
trait HeapLen: DnsLabels {
    fn heap_len(&self) -> usize {
        1 + self.labs() + self.nlen()
    }
}

impl<N> HeapLen for N where N: DnsLabels {}

impl From<&ScratchName> for HeapName {
    fn from(scratch: &ScratchName) -> HeapName {
        let mut vec = Vec::with_capacity(scratch.heap_len());
        vec.push(scratch.labs() as u8);
        vec.extend_from_slice(scratch.lpos());
        vec.extend_from_slice(scratch.name());
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { HeapName::from_vec(vec) }
    }
}

impl From<ScratchName> for HeapName {
    fn from(scratch: ScratchName) -> HeapName {
        HeapName::from(&scratch)
    }
}

impl<P> From<&WireLabels<'_, P>> for HeapName
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn from(wire: &WireLabels<'_, P>) -> HeapName {
        let mut vec = vec![0; wire.heap_len()];
        let labs = wire.labs();
        vec[0] = labs as u8;
        let mut lpos = 0;
        let mut npos = labs + 1;
        for lab in 0..labs {
            let label = wire.label(lab).unwrap();
            let llen = label.len() as u8;
            vec[lab + 1] = lpos;
            lpos += 1 + llen;
            vec[npos] = llen;
            npos += 1;
            for ch in label {
                vec[npos] = ch.to_ascii_lowercase();
                npos += 1;
            }
        }
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { HeapName::from_vec(vec) }
    }
}

impl<P> From<WireLabels<'_, P>> for HeapName
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn from(wire: WireLabels<'_, P>) -> HeapName {
        HeapName::from(&wire)
    }
}

impl TryFrom<&[u8]> for HeapName {
    type Error = Error;
    fn try_from(wire: &[u8]) -> Result<HeapName> {
        let mut scratch = ScratchName::new();
        scratch.from_wire(wire, 0)?;
        Ok(scratch.into())
    }
}

impl TryFrom<&str> for HeapName {
    type Error = Error;
    fn try_from(text: &str) -> Result<HeapName> {
        let mut scratch = ScratchName::new();
        let end = scratch.from_text(text.as_bytes())?;
        if end == text.len() {
            Ok(scratch.into())
        } else {
            Err(NameTrailing)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<()> {
        assert_eq!(".", format!("{}", HeapName::try_from(".")?));
        let text = "dotat.at";
        let name = HeapName::try_from(text)?;
        assert_eq!(text, format!("{}", name));
        let text2 = "dotat.at.";
        let name = HeapName::try_from(text2)?;
        assert_eq!(text, format!("{}", name));
        Ok(())
    }
}
