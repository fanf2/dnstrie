use crate::error::Error::*;
use crate::error::*;
use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;

/// Maximum length of a DNS name, in octets on the wire.
pub const MAX_OCTET: usize = 255;

/// Maximum number of labels in a DNS name.
///
/// Calculated by removing one octet for the root zone, dividing by
/// the smallest possible label, then adding back the root.
///
pub const MAX_LABEL: usize = (MAX_OCTET - 1) / 2 + 1;

/// A buffer for collecting label positions when parsnig a DNS name.
///
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

impl<'n, I> BorrowName<'n, I>
where
    I: Copy + Default,
{
    /// Construct an empty name onto which we will [`push_label()`]
    fn new(buf: &'n [u8]) -> Self {
        let o = Default::default();
        BorrowName { buf, len: 0, labs: 0, lpos: [o; MAX_LABEL] }
    }

    /// Add a label to the name, located in the `buf` at the given
    /// position with the given length.
    fn push_label(&mut self, lpos: I, llen: u8) -> Result<usize> {
        let step = 1 + llen as usize;
        self.lpos[self.labs] = lpos;
        self.labs += 1;
        self.len += step;
        if self.labs >= MAX_LABEL {
            return Err(NameLabels);
        }
        if self.len >= MAX_OCTET {
            return Err(NameLength);
        }
        Ok(step)
    }

    /// Get the label length byte at the given position, and do some
    /// basic checking.
    fn get_llen(&self, pos: usize) -> Result<u8> {
        match *self.buf.get(pos).ok_or(NameTruncated)? {
            byte @ 0x00..=0x3F => Ok(byte),
            byte @ 0x40..=0xBF => Err(LabelType(byte)),
            byte @ 0xC0..=0xFF => Ok(byte),
        }
    }
}

/// Parse a DNS name in uncompressed wire format.
///
/// The resulting `WireName` borrows the `label` and `octet` arguments.
///
pub fn from_wire(buf: &[u8]) -> Result<WireName> {
    let mut name = WireName::new(buf);
    let mut pos = 0;
    loop {
        let llen = name.get_llen(pos)?;
        if let 0xC0..=0xFF = llen {
            return Err(CompressBan);
        }
        pos += name.push_label(pos.try_into()?, llen)?;
        if llen == 0 {
            name.buf = &buf[0..name.len];
            return Ok(name);
        }
    }
}

/// Parse a DNS name in compressed wire format.
///
/// The name starts at the given `pos` in the `msg`.
///
/// The resulting `MessageName` borrows the `label` and `msg`
/// arguments.
///
pub fn from_message(buf: &[u8], mut pos: usize) -> Result<MessageName> {
    let mut name = MessageName::new(buf);
    let mut hwm = pos;
    loop {
        let llen = name.get_llen(pos)?;
        if let hi @ 0xC0..=0xFF = llen {
            let lo = *buf.get(pos + 1).ok_or(NameTruncated)?;
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
        pos += name.push_label(pos.try_into()?, llen)?;
        if llen == 0 {
            return Ok(name);
        }
    }
}

/// A DNS name in wire format.
///
/// This trait covers the methods that are common to [`HeapName`],
/// [`MessageName`], and [`WireName`]
///
pub trait DnsName<'n> {
    /// The length of the name in uncompressed wire format
    fn namelen(self) -> usize;

    /// The number of labels in the name
    fn labels(self) -> usize;

    /// A slice covering a label's length byte and its text
    ///
    /// Returns `None` if the label number is out of range.
    ///
    fn label(self, lab: usize) -> Option<&'n [u8]>;

    /// Returns [`LabelIter`], an iterator visiting each label in a `DnsName`
    fn label_iter(self) -> LabelIter<'n, Self>
    where
        Self: Sized,
    {
        LabelIter { name: self, lab: 0, pos: 0, _elem: PhantomData }
    }

    fn to_text(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        Self: Sized + Copy,
    {
        if self.labels() == 1 {
            write!(f, ".")
        } else {
            for (lab, _, label) in self.label_iter() {
                write_text(f, &label[1..])?;
                // only print the root if this name is the root
                if lab == 0 || label.len() > 1 {
                    write!(f, ".")?;
                }
            }
            Ok(())
        }
    }
}

fn write_text(
    f: &mut std::fmt::Formatter<'_>,
    bytes: &[u8],
) -> std::fmt::Result {
    for &byte in bytes.iter() {
        match byte {
            b'*' | b'-' | b'_' | // permitted punctuation
            b'0'..=b'9' |
            b'A'..=b'Z' |
            b'a'..=b'z' => write!(f, "{}", byte as char)?,
            b'!'..=b'~' => write!(f, "\\{}", byte as char)?,
            _ => write!(f, "\\{:03}", byte)?,
        }
    }
    Ok(())
}

impl<'n, I> std::fmt::Display for BorrowName<'n, I>
where
    I: Into<usize> + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_text(f)
    }
}

impl std::fmt::Display for HeapName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_text(f)
    }
}

/// Helper function to get a slice covering a label
///
/// The slice covers both the length byte and the label text.
///
fn label_slice(buf: &[u8], start: usize) -> &[u8] {
    let len = buf[start] as usize;
    let end = start + len;
    &buf[start..=end]
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
        let pos: usize = self.lpos.get(lab).copied()?.into();
        Some(label_slice(self.buf, pos))
    }
}

/// An iterator visiting each label in a DNS name
///
/// The iterator yeilds a tuple containing:
///
///   * the label number;
///   * the position of the label in the name;
///   * a slice covering the label length byte and the label text.
///
/// Returned by [`DnsName::label_iter()`]
///
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
        if let Some(label) = self.name.label(self.lab) {
            let lab = self.lab;
            let pos = self.pos;
            self.lab += 1;
            self.pos += label.len();
            Some((lab, pos, label))
        } else {
            None
        }
    }
}

/// A DNS name in wire format, owned and allocated on the heap.
///
/// A `HeapName` is intended to be reasonably efficient:
///
///   * it includes an index of the label positions, so it doesn't
///     need to be re-parsed;
///
///   * the label index and name share a single allocation;
///
///   * TODO: it only uses a single word to refer to the allocation.
///
/// The maximum heap allocation is the maximum length of a DNS name
/// (255 bytes) plus the maximum number of labels (128 bytes,
/// including the root), plus a byte for the label count, totalling
/// 384 bytes.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeapName {
    mem: Box<[u8]>,
}

impl<'n> DnsName<'n> for &'n HeapName {
    fn labels(self) -> usize {
        self.mem[0] as usize
    }

    fn namelen(self) -> usize {
        self.mem[self.labels()] as usize + 1
    }

    fn label(self, lab: usize) -> Option<&'n [u8]> {
        let labs = Some(self.labels()).filter(|&labs| lab < labs)?;
        let pos = self.mem[1 + lab] as usize;
        Some(label_slice(&self.mem, 1 + labs + pos))
    }
}

impl<'n> From<&WireName<'n>> for HeapName {
    fn from(wire: &WireName<'n>) -> HeapName {
        let labs = wire.labs;
        let len = wire.len;
        let mut v = vec![0u8; 1 + labs + len];
        v[0] = labs as u8;
        v[1..=labs].copy_from_slice(&wire.lpos[0..labs]);
        v[labs + 1..].copy_from_slice(wire.buf);
        HeapName { mem: v.into_boxed_slice() }
    }
}

impl<'n> From<&MessageName<'n>> for HeapName {
    fn from(msg: &MessageName<'n>) -> HeapName {
        let labs = msg.labs;
        let len = msg.len;
        let mut v = vec![0u8; 1 + labs + len];
        v[0] = labs as u8;
        for (lab, pos, label) in msg.label_iter() {
            v[1 + lab] = pos as u8;
            let start = 1 + labs + pos;
            let end = start + label.len();
            v[start..=end].copy_from_slice(label);
        }
        HeapName { mem: v.into_boxed_slice() }
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
        Ok(HeapName::from(&from_wire(&v[..])?))
    }
}
