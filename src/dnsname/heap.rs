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

use crate::dnsname::*;
use std::marker::PhantomData;

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
/// This safety requirement is established by the constructors,
/// `impl FromWire` and `impl From<ScratchName>`. After that point,
/// the name is immutable so it remains safe.
///
pub struct HeapName {
    // We treat this memory as immutable except when it is dropped.
    mem: *mut u8,
    // NOTE: the marker tells dropck that we logically own some bytes
    _marker: PhantomData<u8>,
}

impl Drop for HeapName {
    fn drop(&mut self) {
        let len = self.heap_len();
        // SAFETY: see [`HeapName`] under "Safety"
        let _ = unsafe { Vec::from_raw_parts(self.mem, len, len) };
    }
}

/// SAFETY: the data in a [`HeapName`] is unaliased.
unsafe impl Send for HeapName {}

/// SAFETY: the data in a [`HeapName`] is unaliased.
unsafe impl Sync for HeapName {}

impl DnsLabels<u8> for HeapName {
    fn labs(&self) -> usize {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { self.mem.read() as usize }
    }

    fn lpos(&self) -> &[u8] {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe {
            let lpos = self.mem.add(1);
            std::slice::from_raw_parts(lpos, self.labs())
        }
    }

    fn nlen(&self) -> usize {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { self.mem.add(self.labs()).read() as usize + 1 }
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

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let pos = *self.lpos().get(lab)? as usize;
        Some(slice_label(self.name(), pos))
    }
}

impl std::fmt::Display for HeapName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_text(f)
    }
}

/// Calculate the allocation size for a [`HeapName`],
///
/// This is just a small extension to the [`DnsLabels`] trait,
/// specific to the needs of a [`HeapName`].
///
trait HeapLen<P>: DnsLabels<P> {
    fn heap_len(&self) -> usize {
        1 + self.labs() + self.nlen()
    }
}

impl<P, N: DnsLabels<P>> HeapLen<P> for N {}

impl From<ScratchName> for HeapName {
    fn from(scratch: ScratchName) -> HeapName {
        unimplemented!()
    }
}
