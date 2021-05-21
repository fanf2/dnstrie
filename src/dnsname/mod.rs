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

pub(self) mod labels;

use crate::error::Error::*;
use crate::error::*;
use crate::scratchpad::ScratchPad;
use std::convert::TryInto;

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

/// A DNS name where we only have information about the labels.
///
/// `WireLabels` are not able to implement all the [`DnsName`]
/// methods, so this supertrait includes the ones that it can.
///
/// The generic parameter is because `WireLabels` can store label
/// positions as `u8` or `u16`.
///
pub trait DnsLabels<P> {
    /// The number of labels in the name
    fn labs(&self) -> usize;

    /// A slice containing the positions of the labels in the name.
    fn lpos(&self) -> &[P];

    /// The length of the name in uncompressed wire format
    fn nlen(&self) -> usize;
}

/// A DNS name in uncompressed lowercase wire format.
///
pub trait DnsName: DnsLabels<u8> {
    /// A slice covering the name.
    fn name(&self) -> &[u8];

    /// A slice covering a label's length byte and its text
    ///
    /// Returns `None` if the label number is out of range.
    ///
    fn label(&self, lab: usize) -> Option<&[u8]>;

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

pub trait FromWire {
    /// Reset the name's scratch pad to empty.
    fn clear(&mut self);

    /// Parse a DNS name from wire format.
    ///
    /// To parse a compressed name in a DNS message, the `wire` slice
    /// should cover the whole message, or if the name is inside a
    /// record's RDATA, a slice from the start of the message to the
    /// end of the RDATA. The `pos` should be the index of the start
    /// of the name in the message.
    ///
    /// To parse a name when compression is not allowed, the slice
    /// should extend from the start of the name to whatever limit
    /// applies, and `pos` should be zero.
    ///
    /// When parsing succeeds, the return value is the position of the
    /// next byte after the name.
    ///
    fn from_wire(&mut self, wire: &[u8], pos: usize) -> Result<usize>;
}

pub trait FromText: FromWire {
    fn from_text(&mut self, text: &[u8]) -> Result<usize>;
}

/// Wrapper for panic-free indexing into untrusted data
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Dodgy<'u> {
    bytes: &'u [u8],
}

impl Dodgy<'_> {
    fn get(self, pos: usize) -> Result<u8> {
        self.bytes.get(pos).map_or(Err(NameTruncated), |p| Ok(*p))
    }

    fn len(self) -> usize {
        self.bytes.len()
    }

    fn fun<F, T>(
        fun: F,
        this: &mut T,
        bytes: &[u8],
        pos: usize,
    ) -> Result<usize>
    where
        F: Fn(&mut T, Dodgy, usize) -> Result<usize>,
        T: FromWire,
    {
        this.clear();
        let ret = fun(this, Dodgy { bytes }, pos);
        if ret.is_err() {
            this.clear();
        }
        ret
    }
}

/// Internal helper trait for [`FromWire`]
///
trait LabelFromWire: FromWire {
    fn label_from_wire(
        &mut self,
        bytes: Dodgy,
        pos: usize,
        llen: u8,
    ) -> Result<()>;
}

fn name_from_wire<T>(this: &mut T, wire: Dodgy, mut pos: usize) -> Result<usize>
where
    T: LabelFromWire,
{
    let mut max = pos;
    let mut end = pos;
    loop {
        let llen = match wire.get(pos)? {
            len @ 0x00..=0x3F => len,
            wat @ 0x40..=0xBF => return Err(LabelType(wat)),
            hi @ 0xC0..=0xFF => {
                end = std::cmp::max(end, pos + 2);
                let lo = wire.get(pos + 1)?;
                pos = (hi as usize & 0x3F) << 8 | lo as usize;
                if let 0xC0..=0xFF = wire.get(pos)? {
                    return Err(CompressChain);
                } else if max <= pos {
                    return Err(CompressBad);
                } else {
                    max = pos;
                    continue;
                }
            }
        };
        this.label_from_wire(wire, pos, llen)?;
        pos += 1 + llen as usize;
        end = std::cmp::max(end, pos);
        if llen == 0 {
            return Ok(end);
        }
    }
}

fn name_from_text<T>(this: &mut T, text: Dodgy, mut pos: usize) -> Result<usize>
where
    T: LabelFromWire,
{
    type ScratchLabel = ScratchPad<u8, MAX_LLEN>;
    let mut label = ScratchLabel::new();
    let mut root = 0;
    let mut sub = 0;

    let mut append_label = |what: Option<&mut ScratchLabel>| {
        if let Some(label) = what {
            let wire = Dodgy { bytes: label.as_slice() };
            let ret = this.label_from_wire(wire, 0, label.len() as u8);
            root += label.is_empty() as usize;
            sub += !label.is_empty() as usize;
            label.clear();
            ret
        } else if root > 1 || (root > 0 && sub > 0) || (root == 0 && sub == 0) {
            Err(NameSyntax)
        } else if root == 0 {
            this.label_from_wire(Dodgy { bytes: &[] }, 0, 0)
        } else {
            Ok(())
        }
    };

    while let Ok(byte) = text.get(pos) {
        match byte {
            // RFC 1035 zone file special characters terminate the name
            b'\n' | b'\r' | b'\t' | b' ' | b';' | b'(' | b')' => break,
            // RFC 1035 suggests that a label can be a quoted string,
            // but it seems better to treat that as an error
            b'"' => return Err(NameSyntax),
            // RFC 1035 peculiar decimal (not octal!) escapes
            b'\\' => {
                pos += 1;
                let mut num = None;
                for _ in 0..=2 {
                    if let Ok(byte @ b'0'..=b'9') = text.get(pos) {
                        let digit = (byte - b'0') as u16;
                        num = Some(num.unwrap_or(0) * 10 + digit);
                        pos += 1;
                    }
                }
                if let Some(code) = num {
                    let byte = code.try_into().or(Err(EscapeBad(code)))?;
                    label.push(byte)?;
                } else {
                    label.push(text.get(pos)?)?;
                }
            }
            // label delimiter
            b'.' => append_label(Some(&mut label))?,
            // everything else
            _ => label.push(byte)?,
        }
    }

    // last label lacked a trailing dot
    if !label.is_empty() {
        append_label(Some(&mut label))?;
    }
    append_label(None).and(Ok(pos))
}

/// Helper function to get a slice covering a label
///
/// The slice covers both the length byte and the label text.
///
/// This is for use with names that have been parsed.
///
/// # Panics
///
/// Panics if the label extends past the end of the name.
///
/// Doesn't check the high bits of the label length byte.
///
fn slice_label(name: &[u8], start: usize) -> &[u8] {
    let llen = name[start] as usize;
    let end = start + llen;
    &name[start..=end]
}
