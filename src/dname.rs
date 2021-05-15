use crate::error::Error::*;
use crate::error::*;
use std::convert::TryInto;

/// Maximum length of a DNS name, in octets on the wire.
pub const MAX_OCTET: usize = 255;

/// Maximum number of labels in a DNS name.
///
/// Calculated by removing one octet for the root zone, dividing by
/// the smallest possible label, then adding back the root.
///
pub const MAX_LABEL: usize = (MAX_OCTET - 1) / 2 + 1;

/// Number of entries in a [`LabelIndex`]
///
pub const LABEL_INDICES: usize = MAX_LABEL + 1;

/// A slice containing the positions of a DNS name's labels
///
/// The first element is the actual number of labels, so that we can
/// create a fixed-size array before we parse the name.
///
/// This is a generic type so that variants can be used by
/// [`WireName`] and [`MessageName`].
///
pub type LabelIndex<I> = [I; LABEL_INDICES];

/// A DNS name that borrows a [`LabelIndex`] and the name itself.
///
/// This allows a DNS name to be parsed from the wire without
/// allocating ot copying.
///
pub struct BorrowName<'i, 'n, I> {
    label: &'i LabelIndex<I>,
    octet: &'n [u8],
}

/// An uncompressed DNS name parsed from wire format.
///
/// Uncompressed names need less space for their [`LabelIndex`], and
/// they borrow a slice that covers the name's bytes and no more.
///
pub type WireName<'i, 'n> = BorrowName<'i, 'n, u8>;

/// A compressed DNS name parsed from a DNS message.
///
/// Compressed names need more space for their [`LabelIndex`], and
/// they borrow a slice covering the whole message.
///
pub type MessageName<'i, 'n> = BorrowName<'i, 'n, u16>;

pub fn from_wire<'i, 'n>(
    label: &'i mut LabelIndex<u8>,
    octet: &'n [u8],
) -> Result<WireName<'i, 'n>> {
    let mut pos = 0;
    let mut lab = 1;
    loop {
        let lablen = match octet.get(pos) {
            Some(0) => break, // root
            Some(byte @ 1..=0x3F) => *byte,
            Some(byte @ 0x40..=0xBF) => return Err(LabelType(*byte)),
            Some(0xC0..=0xFF) => return Err(Compression),
            None => return Err(NameFormat("truncated")),
        };
        label[lab] = pos.try_into()?;
        lab += 1;
        if lab >= LABEL_INDICES {
            return Err(NameFormat("too many labels"));
        }
        pos += 1 + lablen as usize;
        if pos >= MAX_OCTET {
            return Err(NameFormat("too long"));
        }
    }
    label[lab] = pos.try_into()?;
    label[0] = lab.try_into()?;
    Ok(WireName { label, octet: &octet[0..=pos] })
}
