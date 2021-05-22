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

pub mod heap;
pub use self::heap::*;

pub mod scratch;
pub use self::scratch::*;

use crate::error::Error::*;
use crate::error::{Error, Result};

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

/// A DNS name in uncompressed lowercase wire format.
///
pub trait DnsName {
    /// The number of labels in the name
    fn labs(&self) -> usize;

    /// A slice containing the positions of the labels in the name.
    fn lpos(&self) -> &[u8];

    /// A slice covering the name.
    fn name(&self) -> &[u8];

    /// The length of the name in uncompressed wire format
    fn nlen(&self) -> usize;

    /// A slice covering a label's length byte and its text
    ///
    /// Returns `None` if the label is out of range.
    ///
    fn label(&self, lab: usize) -> Option<&[u8]> {
        let start = *self.lpos().get(lab)? as usize;
        let llen = *self.name().get(start)? as usize;
        let end = start + llen;
        self.name().get(start..=end)
    }

    fn to_text(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labs = self.labs();
        for lab in 0..labs {
            let label = self.label(lab).ok_or(std::fmt::Error)?;
            let text = &label[1..];
            for &byte in text.iter() {
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
}
