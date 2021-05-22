//! Temporary copy of a DNS name
//! ============================
//!
//! This kind of name is decompressed and canonicalized to lower case.
//! The name and label pointers are stored in its scratch pad.

use crate::dnsname::*;
use crate::scratchpad::*;
use core::convert::TryInto;

#[derive(Debug, Default)]
pub struct ScratchName {
    lpos: ScratchPad<u8, MAX_LABS>,
    name: ScratchPad<u8, MAX_NAME>,
}

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

impl std::fmt::Display for ScratchName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_text(f)
    }
}

impl Eq for ScratchName {}

impl<Other: DnsName> PartialEq<Other> for ScratchName {
    fn eq(&self, other: &Other) -> bool {
        self.name() == other.name()
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

    pub fn from_text(&mut self, text: &[u8]) -> Result<usize> {
        let dodgy = Dodgy { bytes: text };
        self.dodgy_from_text(dodgy).map_err(|err| self.clear_err(err))
    }

    pub fn from_wire(&mut self, wire: &[u8], pos: usize) -> Result<usize> {
        let dodgy = Dodgy { bytes: wire };
        self.dodgy_from_wire(dodgy, pos).map_err(|err| self.clear_err(err))
    }

    fn add_label(&mut self, dodgy: Dodgy, rpos: usize, llen: u8) -> Result<()> {
        let wpos = self.nlen().try_into().or(Err(NameLengthWat))?;
        self.lpos.push(wpos)?;
        self.name.push(llen)?;
        for i in 0..llen as usize {
            self.name.push(dodgy.get(rpos + i)?.to_ascii_lowercase())?;
        }
        Ok(())
    }

    fn dodgy_from_text(&mut self, dodgy: Dodgy) -> Result<usize> {
        type ScratchLabel = ScratchPad<u8, MAX_LLEN>;
        let mut label = ScratchLabel::new();
        let mut root = 0;
        let mut sub = 0;

        let mut check_or_add = |what: Option<&mut ScratchLabel>| {
            if let Some(label) = what {
                let len = label.len().try_into().or(Err(LabelLengthWat))?;
                let dodgy = Dodgy { bytes: label.as_slice() };
                let ret = self.add_label(dodgy, 0, len);
                root += label.is_empty() as usize;
                sub += !label.is_empty() as usize;
                label.clear();
                ret
            } else if root > 1
                || (root > 0 && sub > 0)
                || (root == 0 && sub == 0)
            {
                Err(NameSyntax)
            } else if root == 0 {
                self.add_label(Dodgy { bytes: &[] }, 0, 0)
            } else {
                Ok(())
            }
        };

        let mut pos = 0;
        while let Ok(byte) = dodgy.get(pos) {
            pos += 1;
            match byte {
                // RFC 1035 suggests that a label can be a quoted string,
                // but it seems better to treat that as an error
                b'"' => return Err(NameSyntax),
                // RFC 1035 zone file special characters terminate the name
                b'\n' | b'\r' | b'\t' | b' ' | b';' | b'(' | b')' => {
                    pos -= 1; // unget
                    break;
                }
                // RFC 1035 peculiar decimal (not octal!) escapes
                b'\\' => {
                    let mut num = None;
                    for _ in 1..=3 {
                        if let Ok(byte @ b'0'..=b'9') = dodgy.get(pos) {
                            let digit = (byte - b'0') as u16;
                            num = Some(num.unwrap_or(0) * 10 + digit);
                            pos += 1;
                        }
                    }
                    if let Some(code) = num {
                        let byte = code.try_into().or(Err(EscapeBad(code)))?;
                        label.push(byte)?;
                    } else {
                        label.push(dodgy.get(pos)?)?;
                        pos += 1;
                    }
                }
                // label delimiter
                b'.' => check_or_add(Some(&mut label))?,
                // everything else
                _ => label.push(byte)?,
            }
        }

        // last label lacked a trailing dot
        if !label.is_empty() {
            check_or_add(Some(&mut label))?;
        }
        check_or_add(None).and(Ok(pos))
    }

    fn dodgy_from_wire(&mut self, dodgy: Dodgy, pos: usize) -> Result<usize> {
        let mut pos = pos;
        let mut max = pos;
        let mut end = pos;
        loop {
            let llen = match dodgy.get(pos)? {
                len @ 0x00..=0x3F => len,
                wat @ 0x40..=0xBF => return Err(LabelType(wat)),
                hi @ 0xC0..=0xFF => {
                    end = std::cmp::max(end, pos + 2);
                    let lo = dodgy.get(pos + 1)?;
                    pos = (hi as usize & 0x3F) << 8 | lo as usize;
                    if let 0xC0..=0xFF = dodgy.get(pos)? {
                        return Err(CompressChain);
                    } else if max <= pos {
                        return Err(CompressBad);
                    } else {
                        max = pos;
                        continue;
                    }
                }
            };
            self.add_label(dodgy, pos + 1, llen)?;
            pos += 1 + llen as usize;
            end = std::cmp::max(end, pos);
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

impl Dodgy<'_> {
    fn get(self, pos: usize) -> Result<u8> {
        self.bytes.get(pos).map_or(Err(NameTruncated), |p| Ok(*p))
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
