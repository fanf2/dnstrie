//! DNS names
//! =========
//!
//! buffering temporary names
//! -------------------------
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
//! We would like to make these reformatted names without allocating.
//! There is a common `dnsname::Buffer` type which is a fixed size
//! large enough for any DNS-name related purpose.
//!
//! The caller creates a buffer on the stack and passes it to a
//! function that needs it. This is a manual return value
//! optimization, because Rust does not yet do that automatically.
//!
//! Functions that use a `Buffer` retain a reference to it as part of
//! the return value, in effect taking ownership without the move that
//! that implies.

pub mod temp;
pub mod workpad;

pub use self::temp::*;
pub(self) use self::workpad::*;

use crate::error::Error::*;
use crate::error::*;

/// Maximum length of a DNS name, in octets on the wire.
pub const MAX_OCTET: usize = 255;

/// Maximum number of labels in a DNS name.
///
/// Calculated by removing one octet for the root zone, dividing by
/// the smallest possible label, then adding back the root.
///
pub const MAX_LABEL: usize = (MAX_OCTET - 1) / 2 + 1;

/// A DNS name in wire format.
///
/// This trait has the functions common to DNS names that own the name
/// data (i.e. not `WireName`)
///
pub trait DnsName {
    /// The length of the name in uncompressed wire format
    fn namelen(&self) -> usize;

    /// The number of labels in the name
    fn labels(&self) -> usize;

    /// A slice covering a label's length byte and its text
    ///
    /// Returns `None` if the label number is out of range.
    ///
    fn label(&self, lab: usize) -> Option<&[u8]>;

    fn to_text(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labs = self.labels();
        for lab in 0..labs {
            let label = self.label(lab).unwrap();
            let text = &label[1..];
            for &byte in text.iter() {
                match byte {
                    b'*' | b'-' | b'_' | // permitted punctuation
                    b'0'..=b'9' |
                    b'A'..=b'Z' |
                    b'a'..=b'z' => write!(f, "{}", byte as char)?,
                    b'!'..=b'~' => write!(f, "\\{}", byte as char)?,
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

pub trait DnsNameParser {
    /// Parse a DNS name from wire format.
    ///
    fn from_wire(&mut self, decomp: Option<usize>, wire: &[u8]) -> Result<()> {
        let mut pos = decomp.unwrap_or(0);
        let mut max = pos;
        let mut len = 0;
        let mut labs = 0;
        loop {
            let llen = match *wire.get(pos).ok_or(NameTruncated)? {
                len @ 0x00..=0x3F => len,
                wat @ 0x40..=0xBF => return Err(LabelType(wat)),
                hi @ 0xC0..=0xFF => {
                    if decomp == None {
                        return Err(CompressBan);
                    }
                    let lo = *wire.get(pos + 1).ok_or(NameTruncated)?;
                    pos = (hi as usize & 0x3F) << 8 | lo as usize;
                    if pos >= max {
                        return Err(CompressWild);
                    }
                    max = pos;
                    if let Some(0xC0..=0xFF) = wire.get(pos) {
                        return Err(CompressChain);
                    }
                    continue;
                }
            };
            let step = 1 + llen as usize;
            if labs + 1 > MAX_LABEL {
                return Err(NameLabels);
            }
            if len + step > MAX_OCTET {
                return Err(NameLength);
            }
            self.parsed_label(wire, pos, llen)?;
            if llen > 0 {
                labs += 1;
                len += step;
                pos += step;
            } else {
                return Ok(());
            }
        }
    }

    /// Add a label to the work pad, located on the `wire` at the
    /// given position with the given length.
    fn parsed_label(
        &mut self,
        wire: &[u8],
        lpos: usize,
        llen: u8,
    ) -> Result<()>;
}

/// Helper function to get a slice covering a label
///
/// The slice covers both the length byte and the label text.
///
/// # Panics
///
/// Panics if the label extends past the end of the octets.
///
/// Doesn't check the high bits of the label length byte.
///
fn slice_label(octet: &[u8], start: usize) -> &[u8] {
    let len = octet[start] as usize;
    let end = start + len;
    &octet[start..=end]
}
