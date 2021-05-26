//! Temporary copy of a DNS name
//! ============================
//!
//! This kind of name is decompressed and canonicalized to lower case.
//! The name and label pointers are stored in its scratch pad.

use crate::dnsname::*;
use crate::scratchpad::*;
use core::cmp::max;
use core::convert::TryInto;
use std::str::{from_utf8, FromStr};

#[derive(Debug, Default)]
pub struct ScratchName {
    lpos: ScratchPad<u8, MAX_LABS>,
    name: ScratchPad<u8, MAX_NAME>,
}

impl_dns_name!(ScratchName);

impl DnsName for ScratchName {
    fn labs(&self) -> usize {
        self.lpos.len()
    }

    fn lpos(&self) -> &[u8] {
        self.lpos.as_slice()
    }

    fn name(&self) -> &[u8] {
        self.name.as_slice()
    }

    fn nlen(&self) -> usize {
        self.name.len()
    }
}

impl ScratchName {
    #[inline(always)]
    pub fn new() -> Self {
        ScratchName { lpos: ScratchPad::new(), name: ScratchPad::new() }
    }

    pub fn clear(&mut self) {
        self.lpos.clear();
        self.name.clear();
    }

    fn clear_err(&mut self, err: Error) -> Error {
        self.clear();
        err
    }

    pub fn from_wire(&mut self, wire: &[u8], pos: usize) -> Result<usize> {
        let dodgy = Dodgy { bytes: wire };
        self.clear();
        self.dodgy_from_wire(dodgy, pos).map_err(|err| self.clear_err(err))
    }

    pub fn from_text(&mut self, text: &[u8]) -> Result<usize> {
        let dodgy = Dodgy { bytes: text };
        self.clear();
        self.dodgy_from_text(dodgy).map_err(|err| self.clear_err(err))
    }

    fn add_label(&mut self, dodgy: Dodgy, rpos: usize, llen: u8) -> Result<()> {
        let wpos = self.nlen().try_into()?; // u8 > MAX_NAME
        self.lpos.push(wpos)?;
        self.name.push(llen)?;
        for i in 0..llen as usize {
            self.name.push(dodgy.get(rpos + i)?.to_ascii_lowercase())?;
        }
        Ok(())
    }

    fn dodgy_from_wire(&mut self, dodgy: Dodgy, pos: usize) -> Result<usize> {
        let mut pos = pos;
        let mut hwm = pos;
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
                    } else if hwm <= pos {
                        return Err(CompressBad);
                    } else {
                        hwm = pos;
                        continue;
                    }
                }
            };
            self.add_label(dodgy, pos + 1, llen)?;
            pos += 1 + llen as usize;
            end = max(end, pos);
            if llen == 0 {
                return Ok(end);
            }
        }
    }

    fn dodgy_from_text(&mut self, dodgy: Dodgy) -> Result<usize> {
        let mut label = ScratchPad::<u8, MAX_LLEN>::new();
        let mut root = 0;
        let mut pos = 0;
        while label_from_text(&mut label, dodgy, &mut pos)? {
            let llen = label.len().try_into()?; // u8 > MAX_LLEN
            let sound = Dodgy { bytes: label.as_slice() };
            self.add_label(sound, 0, llen)?;
            root += (llen == 0) as usize;
        }
        if root > 1 || (root > 0 && self.labs() > 1) || self.labs() == 0 {
            return Err(NameSyntax);
        } else if root == 0 {
            self.add_label(Dodgy { bytes: &[] }, 0, 0)?;
        }
        Ok(pos)
    }
}

fn label_from_text(
    label: &mut ScratchPad<u8, MAX_LLEN>,
    dodgy: Dodgy,
    pos: &mut usize,
) -> Result<bool> {
    label.clear();
    while let Ok(byte) = dodgy.get(*pos) {
        *pos += 1;
        match byte {
            b'\\' => match dodgy.get(*pos)? {
                // RFC 1035 peculiar decimal (not octal!) escapes
                b'0'..=b'9' => {
                    let code = dodgy.slice(*pos, 3)?;
                    label.push(u8::from_str(from_utf8(code)?)?)?;
                    *pos += 3;
                }
                esc => {
                    label.push(esc)?;
                    *pos += 1;
                }
            },
            // RFC 1035 suggests that a label can be a quoted string,
            // but it seems better to treat that as an error
            b'"' => return Err(NameSyntax),
            // terminated by RFC 1035 zone file special characters
            b'\n' | b'\r' | b'\t' | b' ' | b';' | b'(' | b')' => {
                *pos -= 1; // unget terminator
                return Ok(!label.is_empty());
            }
            // always add a label when we see a delimiter
            b'.' => return Ok(true),
            // everything else
            _ => label.push(byte)?,
        }
    }
    Ok(!label.is_empty())
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

#[cfg(test)]
mod test {
    use crate::dnsname::*;

    #[test]
    fn test() -> Result<()> {
        let wire = b"\x05dotat\x02at\x00";
        let mut name = ScratchName::new();
        name.from_wire(wire, 0)?;
        assert_eq!("dotat.at", format!("{}", name));
        Ok(())
    }
}
