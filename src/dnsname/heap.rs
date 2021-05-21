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

use crate::dnsname::labels::*;
use crate::dnsname::*;
use crate::error::*;
use std::convert::TryFrom;
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
/// The maximum heap allocation is [`dnsname::MAX_NAME`] plus
/// [`dnsname::MAX_LABS`] plus a byte for the label count, totalling
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
///   * the allocation size matches the [`HeapLen`]
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

impl DnsName for HeapName {
    fn labs(&self) -> usize {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { self.mem.read() as usize }
    }

    fn nlen(&self) -> usize {
        // SAFETY: see [`HeapName`] under "Safety"
        unsafe { self.mem.add(self.labs()).read() as usize + 1 }
    }

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

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let labs = Some(self.labs()).filter(|&labs| lab < labs)?;
        let pos = self.lpos()[1 + lab] as usize;
        Some(slice_label(self.name(), pos))
    }
}

/// Calculate the allocation size for a [`HeapName`],

trait HeapLen: DnsName {
    fn heap_len(&self) -> usize {
        1 + self.labs() + self.nlen()
    }
}

impl<T: DnsName> HeapLen for T {}

impl From<ScratchName> for HeapName {
    fn from(scratch: ScratchName) -> HeapName {
        unimplemented!()
    }
}

impl TryFrom<&str> for HeapName {
    type Error = Error;

    fn try_from(text: &str) -> Result<HeapName> {
        let mut v = Vec::new();

        fn label(v: &mut Vec<u8>, pos: usize) -> Result<usize> {
            if let len @ 0..=0x3F = v.len() - pos {
                v[pos] = len as u8;
                v.push(0);
                Ok(v.len() - 1)
            } else {
                Err(LabelLength)
            }
        }

        let mut pos = label(&mut v, 0)?;
        let mut it = text.as_bytes().iter().peekable();
        while let Some(&byte) = it.next() {
            match byte {
                // RFC 1035 zone file special characters
                b'\n' | b'\r' | b'\t' | b' ' | b';' | b'(' | b')' => break,
                // RFC 1035 suggests that a label can be a quoted
                // string; seems better to treat that as an error
                b'"' => return Err(NameQuotes),
                // RFC 1035 peculiar decimal escapes
                b'\\' => match it.next() {
                    Some(&digit @ b'0'..=b'9') => {
                        let mut n = (digit - b'0') as u16;
                        if let Some(&&digit @ b'0'..=b'9') = it.peek() {
                            n = n * 10 + (digit - b'0') as u16;
                            it.next();
                        }
                        if let Some(&&digit @ b'0'..=b'9') = it.peek() {
                            n = n * 10 + (digit - b'0') as u16;
                            it.next();
                        }
                        let byte = u8::try_from(n)?;
                        v.push(byte);
                    }
                    Some(&byte) => v.push(byte),
                    None => return Err(NameTruncated),
                },
                // label delimiter
                b'.' => pos = label(&mut v, pos)?,
                // RFC 4034 canonical case
                b'A'..=b'Z' => v.push(byte - b'A' + b'a'),
                // everything else
                _ => v.push(byte),
            }
        }
        if pos < v.len() - 1 {
            label(&mut v, pos)?;
        }

        unimplemented!()
    }
}
