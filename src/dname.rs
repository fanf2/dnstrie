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

/// An array which can hold the positions of a DNS name's labels
///
/// This is a generic type so that variants can be used by
/// [`WireName`] and [`MessageName`].
///
pub type LabelBuf<'i, I> = &'i mut [I; MAX_LABEL];

/// A slice holding the positions of a DNS name's labels
///
/// This is a generic type so that variants can be used by
/// [`WireName`] and [`MessageName`].
///
pub type LabelIndex<'i, I> = &'i [I];

/// A DNS name that borrows a [`LabelIndex`] and the name itself.
///
/// This allows a DNS name to be parsed from the wire without
/// allocating ot copying.
///
pub struct BorrowName<'i, 'n, I> {
    label: LabelIndex<'i, I>,
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

/// Parse a DNS name in uncompressed wire format.
///
/// The resulting `WireName` borrows the `label` and `octet` arguments.
///
pub fn from_wire<'i, 'n>(
    label: LabelBuf<'i, u8>,
    octet: &'n [u8],
) -> Result<WireName<'i, 'n>> {
    let mut pos = 0;
    let mut lab = 0;
    loop {
        let lablen = match octet.get(pos) {
            Some(0) => break, // root
            Some(&byte @ 1..=0x3F) => byte,
            Some(&byte @ 0x40..=0xBF) => return Err(LabelType(byte)),
            Some(0xC0..=0xFF) => return Err(CompressBan),
            None => return Err(NameTruncated),
        };
        label[lab] = pos.try_into()?;
        lab += 1;
        if lab >= MAX_LABEL {
            return Err(NameLabels);
        }
        pos += 1 + lablen as usize;
        if pos >= MAX_OCTET {
            return Err(NameLength);
        }
    }
    label[lab] = pos.try_into()?;
    Ok(WireName { label: &label[0..=lab], octet: &octet[0..=pos] })
}

/// Parse a DNS name in compressed wire format.
///
/// The name starts at the given `pos` in the `msg`.
///
/// The resulting `MessageName` borrows the `label` and `msg`
/// arguments.
///
pub fn from_message<'i, 'n>(
    label: LabelBuf<'i, u16>,
    msg: &'n [u8],
    mut pos: usize,
) -> Result<MessageName<'i, 'n>> {
    let mut hwm = pos;
    let mut lab = 0;
    loop {
        let lablen = match msg.get(pos) {
            Some(0) => break, // root
            Some(&byte @ 1..=0x3F) => byte,
            Some(&byte @ 0x40..=0xBF) => return Err(LabelType(byte)),
            Some(&hi @ 0xC0..=0xFF) => match msg.get(pos + 1) {
                Some(&lo) => {
                    pos = (hi as usize & 0x3F) << 8 | lo as usize;
                    if pos >= hwm {
                        return Err(CompressWild);
                    }
                    hwm = pos;
                    if let Some(0xC0..=0xFF) = msg.get(pos) {
                        return Err(CompressChain);
                    }
                    continue;
                }
                None => return Err(NameTruncated),
            },
            None => return Err(NameTruncated),
        };
        label[lab] = pos.try_into()?;
        lab += 1;
        if lab >= MAX_LABEL {
            return Err(NameLabels);
        }
        pos += 1 + lablen as usize;
        if pos >= MAX_OCTET {
            return Err(NameLength);
        }
    }
    label[lab] = pos.try_into()?;
    Ok(MessageName { label: &label[0..=lab], octet: msg })
}
