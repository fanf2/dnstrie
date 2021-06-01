//! DNS names
//! =========
//!
//! terminology
//! -----------
//!
//!   * label: a slice covering a label
//!
//!   * labels: function to get count of labels in a name
//!
//!   * lab: a label number, 0 <= lab < labs
//!
//!   * labs: variable containing count of labels in a name
//!
//!   * llen: length of a label
//!
//!   * lpos: slice of label positions
//!
//!   * name: a slice covering a contiguous (uncompressed) name
//!
//!   * nlen: length of the name
//!
//!   * pos: current position in a slice
//!
//!   * rpos: read position
//!
//!   * wire: a slice of untrustworthy data
//!
//!   * wpos: write position
//!
//! scratch space for temporary names
//! ---------------------------------
//!
//! In some cases we need to reformat a name, and the altered copy is
//! only needed temporarily.
//!
//!   * To ensure a wire-format name is contiguous and in lower case,
//!     to allow fast equality comparisons. We would like to make
//!     these copies in one pass from the wire, which means we don't
//!     know how must space they need in advance.
//!
//!   * To construct a qp-trie key, which is only needed during
//!     lookup. We need to find out where the label boundaries are and
//!     how much the name expands due to escaping, so this requires
//!     two passes, but we want to avoid any more than that.
//!
//! We use [`ScratchPad`][crate::scratchpad::ScratchPad]s to make
//! these reformatted names without allocating.

use crate::error::Error::*;
use crate::error::{Error, Result};
use core::cmp::max;
use core::cmp::Ordering;

pub use self::heap::*;
pub use self::scratch::*;
pub use self::wire::*;

/// Maximum length of a DNS name, in octets on the wire.
pub const MAX_NAME: usize = 255;

/// Maximum length of a DNS label, in octets on the wire.
pub const MAX_LLEN: usize = 0x3F;

/// Maximum number of labels in a DNS name.
///
/// Calculated by removing one octet for the root zone, dividing by
/// the smallest possible label, then adding back the root.
///
pub const MAX_LABS: usize = (MAX_NAME - 1) / 2 + 1;

/// An index of the labels of a DNS name
///
pub trait DnsLabels {
    /// The number of labels in the name
    fn labs(&self) -> usize;

    /// The length of the name in uncompressed wire format
    fn nlen(&self) -> usize;

    /// A slice covering a label's text, counting from 0 on the
    /// left.
    ///
    /// Returns `None` if the label is out of range.
    ///
    fn label(&self, lab: usize) -> Option<&[u8]>;

    /// A slice covering a label's text, counting from the right
    /// where 0 is the root zone.
    ///
    /// Returns `None` if the label is out of range.
    ///
    fn rlabel(&self, lab: usize) -> Option<&[u8]> {
        match self.labs() - 1 {
            root if root >= lab => self.label(root - lab),
            _ => None,
        }
    }

    fn to_text(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labs = self.labs();
        for lab in 0..labs {
            let label = self.label(lab).ok_or(std::fmt::Error)?;
            for &byte in label.iter() {
                match byte {
                    b'*' | b'-' | b'_' | // permitted punctuation
                    b'0'..=b'9' |
                    b'A'..=b'Z' |
                    b'a'..=b'z' => write!(f, "{}", byte as char)?,
                    b'!'..=b'~' => write!(f, "\\{}", byte as char)?,
                    // RFC 1035 peculiar decimal escapes
                    _ => write!(f, "\\{:03}", byte)?,
                }
            }
            if lab == 0 || lab + 2 < labs {
                write!(f, ".")?;
            }
        }
        Ok(())
    }

    fn name_cmp<Other>(&self, other: &Other) -> Ordering
    where
        Other: DnsLabels,
    {
        for lab in 0.. {
            let left = &self.rlabel(lab);
            let right = &other.rlabel(lab);
            match left.cmp(right) {
                Ordering::Equal if left.is_none() && right.is_none() => break,
                Ordering::Equal => continue,
                ne => return ne,
            }
        }
        Ordering::Equal
    }
}

macro_rules! impl_dns_labels {
    ($name:ty : $other:ident) => {
        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.name_cmp(other)
            }
        }

        impl<Other: $other> PartialOrd<Other> for $name {
            fn partial_cmp(&self, other: &Other) -> Option<Ordering> {
                Some(self.name_cmp(other))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.to_text(f)
            }
        }
    };
}

/// A DNS name in uncompressed lowercase wire format.
///
pub trait DnsName: DnsLabels {
    /// A slice covering the name.
    fn name(&self) -> &[u8];

    /// A slice containing the positions of the labels in the name.
    fn lpos(&self) -> &[u8];

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let pos = *self.lpos().get(lab)? as usize;
        let len = *self.name().get(pos)? as usize;
        self.name().get((pos + 1)..=(pos + len))
    }
}

macro_rules! impl_dns_name {
    ($name:ty) => {
        impl Eq for $name {}

        impl<Other: DnsName> PartialEq<Other> for $name {
            fn eq(&self, other: &Other) -> bool {
                self.name() == other.name()
            }
        }

        impl_dns_labels!($name: DnsName);
    };
}

/// Parse a DNS name from the wire
///
pub trait FromWire<'n, 'w> {
    fn from_wire(&'n mut self, wire: &'w [u8], pos: usize) -> Result<usize>;
}

/// Shared implementation for parsing a wire-format DNS name
///
trait LabelFromWire {
    fn label_from_wire(
        &mut self,
        dodgy: Dodgy,
        pos: usize,
        llen: u8,
    ) -> Result<()>;

    fn dodgy_from_wire(&mut self, dodgy: Dodgy, pos: usize) -> Result<usize> {
        let mut pos = pos;
        let mut min = pos;
        let mut end = pos;
        loop {
            let llen = match dodgy.get(pos)? {
                len @ 0x00..=0x3F => len,
                wat @ 0x40..=0xBF => return Err(LabelType(wat)),
                hi @ 0xC0..=0xFF => {
                    end = max(end, pos + 2);
                    let lo = dodgy.get(pos + 1)?;
                    pos = (hi as usize & 0x3F) << 8 | lo as usize;
                    if let 0xC0..=0xFF = dodgy.get(pos)? {
                        return Err(CompressChain);
                    } else if min <= pos {
                        return Err(CompressBad);
                    } else {
                        min = pos;
                        continue;
                    }
                }
            };
            self.label_from_wire(dodgy, pos, llen)?;
            pos += 1 + llen as usize;
            end = max(end, pos);
            if llen == 0 {
                return Ok(end);
            }
        }
    }
}

/// Wrapper for panic-free indexing into untrusted data
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Dodgy<'u> {
    bytes: &'u [u8],
}

impl<'u> Dodgy<'u> {
    fn get(self, pos: usize) -> Result<u8> {
        self.bytes.get(pos).copied().ok_or(NameTruncated)
    }
    fn slice(self, pos: usize, len: usize) -> Result<&'u [u8]> {
        self.bytes.get(pos..pos + len).ok_or(NameTruncated)
    }
}

pub mod heap;
pub mod scratch;
pub mod wire;
