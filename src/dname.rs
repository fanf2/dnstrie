use crate::error::Error::*;
use crate::error::*;
use std::convert::TryInto;
use std::marker::PhantomData;

/// Maximum length of a DNS name, in octets on the wire.
pub const MAX_OCTET: usize = 255;

/// Maximum number of labels in a DNS name.
///
/// Calculated by removing one octet for the root zone, dividing by
/// the smallest possible label, then adding back the root.
///
pub const MAX_LABEL: usize = (MAX_OCTET - 1) / 2 + 1;

type LabelPos<I> = [I; MAX_LABEL];

/// A DNS name that borrows the buffer it was parsed from.
///
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct BorrowName<'n, I> {
    /// where the name lives
    buf: &'n [u8],
    /// total length of the name when uncompressed
    len: usize,
    /// number of labels including the root
    labs: usize,
    /// position in `buf` of the start of each label
    lpos: LabelPos<I>,
}

/// An uncompressed DNS name parsed from wire format.
///
/// Uncompressed names need less space for their label positions, and
/// they borrow a slice that covers the name's bytes and no more.
///
pub type WireName<'n> = BorrowName<'n, u8>;

/// A compressed DNS name parsed from a DNS message.
///
/// Compressed names need more space for their label positions, and
/// they borrow a slice covering the whole message.
///
pub type MessageName<'n> = BorrowName<'n, u16>;

/// Parse a DNS name in uncompressed wire format.
///
/// The resulting `WireName` borrows the `label` and `octet` arguments.
///
pub fn from_wire(buf: &[u8]) -> Result<WireName> {
    let mut lpos = [0; MAX_LABEL];
    let mut pos = 0;
    let mut lab = 0;
    loop {
        let llen = match buf.get(pos) {
            Some(0) => break, // root
            Some(&byte @ 1..=0x3F) => byte,
            Some(&byte @ 0x40..=0xBF) => return Err(LabelType(byte)),
            Some(0xC0..=0xFF) => return Err(CompressBan),
            None => return Err(NameTruncated),
        };
        lpos[lab] = pos.try_into()?;
        lab += 1;
        if lab >= MAX_LABEL {
            return Err(NameLabels);
        }
        pos += 1 + llen as usize;
        if pos >= MAX_OCTET {
            return Err(NameLength);
        }
    }
    lpos[lab] = pos.try_into()?;
    Ok(WireName { buf: &buf[0..=pos], len: pos + 1, labs: lab, lpos })
}

/// Parse a DNS name in compressed wire format.
///
/// The name starts at the given `pos` in the `msg`.
///
/// The resulting `MessageName` borrows the `label` and `msg`
/// arguments.
///
pub fn from_message(buf: &[u8], mut pos: usize) -> Result<MessageName> {
    let mut lpos = [0; MAX_LABEL];
    let mut hwm = pos;
    let mut lab = 0;
    let mut len = 0;
    loop {
        let lablen = match buf.get(pos) {
            Some(0) => break, // root
            Some(&byte @ 1..=0x3F) => byte,
            Some(&byte @ 0x40..=0xBF) => return Err(LabelType(byte)),
            Some(&hi @ 0xC0..=0xFF) => match buf.get(pos + 1) {
                Some(&lo) => {
                    pos = (hi as usize & 0x3F) << 8 | lo as usize;
                    if pos >= hwm {
                        return Err(CompressWild);
                    }
                    hwm = pos;
                    if let Some(0xC0..=0xFF) = buf.get(pos) {
                        return Err(CompressChain);
                    }
                    continue;
                }
                None => return Err(NameTruncated),
            },
            None => return Err(NameTruncated),
        };
        lpos[lab] = pos.try_into()?;
        lab += 1;
        if lab >= MAX_LABEL {
            return Err(NameLabels);
        }
        pos += 1 + lablen as usize;
        len += 1 + lablen as usize;
        if len >= MAX_OCTET {
            return Err(NameLength);
        }
    }
    lpos[lab] = pos.try_into()?;
    Ok(MessageName { len: len + 1, labs: lab, buf, lpos })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedName {
    mem: Box<[u8]>,
}

impl<'n> From<WireName<'n>> for OwnedName {
    fn from(wire: WireName<'n>) -> OwnedName {
        let labs = wire.labs;
        let len = wire.len;
        let mut v = vec![0u8; 1 + labs + len];
        v[0] = labs as u8;
        v[1..=labs].copy_from_slice(&wire.lpos[0..labs]);
        v[labs + 1..].copy_from_slice(wire.buf);
        OwnedName { mem: v.into_boxed_slice() }
    }
}

impl<'n> From<MessageName<'n>> for OwnedName {
    fn from(msg: MessageName<'n>) -> OwnedName {
        let labs = msg.labs;
        let len = msg.len;
        let mut v = vec![0u8; 1 + labs + len];
        v[0] = labs as u8;
        for (lab, pos, label) in msg.label_iter() {
            v[1 + lab] = pos as u8;
            let start = 1 + labs + pos + 1;
            let end = start + label.len();
            v[start - 1] = label.len() as u8;
            v[start..end].copy_from_slice(label);
        }
        OwnedName { mem: v.into_boxed_slice() }
    }
}

pub trait DnsName<'n> {
    fn namelen(self) -> usize;
    fn labels(self) -> usize;
    fn label(self, lab: usize) -> Option<&'n [u8]>;

    fn label_iter(self) -> LabelIter<'n, Self>
    where
        Self: Sized;
}

impl<'n> DnsName<'n> for &'n OwnedName {
    fn labels(self) -> usize {
        self.mem[0] as usize
    }

    fn namelen(self) -> usize {
        self.mem.len() - self.labels() - 1
    }

    fn label(self, lab: usize) -> Option<&'n [u8]> {
        let labs = self.labels();
        if lab < labs {
            let pos = self.mem[1 + lab] as usize;
            let start = 1 + labs + pos + 1;
            let len = self.mem[start - 1] as usize;
            let end = start + len;
            Some(&self.mem[start..end])
        } else {
            None
        }
    }
    fn label_iter(self) -> LabelIter<'n, Self> {
        LabelIter { name: self, lab: 0, pos: 0, _elem: PhantomData }
    }
}

impl<'n, I> DnsName<'n> for &BorrowName<'n, I>
where
    I: Into<usize> + Copy,
{
    fn namelen(self) -> usize {
        self.len
    }

    fn labels(self) -> usize {
        self.labs
    }

    fn label(self, lab: usize) -> Option<&'n [u8]> {
        if lab < self.labels() {
            let pos: usize = self.lpos[lab].into();
            let start = pos + 1;
            let len = self.buf[start - 1] as usize;
            let end = start + len;
            Some(&self.buf[start..end])
        } else {
            None
        }
    }
    fn label_iter(self) -> LabelIter<'n, Self> {
        LabelIter { name: self, lab: 0, pos: 0, _elem: PhantomData }
    }
}

pub struct LabelIter<'n, N> {
    name: N,
    lab: usize,
    pos: usize,
    _elem: PhantomData<&'n [u8]>,
}

impl<'n, N> Iterator for LabelIter<'n, N>
where
    N: DnsName<'n> + Copy,
{
    type Item = (usize, usize, &'n [u8]);
    fn next(&mut self) -> Option<(usize, usize, &'n [u8])> {
        if self.lab < self.name.labels() {
            let lab = self.lab;
            let pos = self.pos;
            let label = self.name.label(lab).unwrap();
            self.lab += 1;
            self.pos += 1 + label.len();
            Some((lab, pos, label))
        } else {
            None
        }
    }
}
