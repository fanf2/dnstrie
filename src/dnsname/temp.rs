//! Temporary copy of a DNS name
//! ============================
//!
//! This kind of name is decompressed and canonicalized to lower case.
//! The name and label pointers are stored in its scratch pad.

use crate::dnsname::*;
use crate::scratchpad::*;
use core::convert::TryInto;

#[derive(Debug, Default)]
pub struct TempName {
    label: ScratchPad<u8, MAX_LABEL>,
    octet: ScratchPad<u8, MAX_OCTET>,
}

impl TempName {
    #[inline(always)]
    pub fn new() -> Self {
        TempName { label: ScratchPad::new(), octet: ScratchPad::new() }
    }
}

impl DnsName for TempName {
    fn namelen(&self) -> usize {
        self.octet.len()
    }

    fn labels(&self) -> usize {
        self.label.len()
    }

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let pos = *self.label.as_slice().get(lab)?;
        Some(slice_label(self.octet.as_slice(), pos as usize))
    }
}

impl DnsNameParser for TempName {
    fn parsed_label(
        &mut self,
        wire: &[u8],
        lpos: usize,
        llen: u8,
    ) -> Result<()> {
        self.label.push(self.octet.len().try_into()?)?;
        self.octet.push(llen)?;
        for i in 1..=llen as usize {
            match wire[lpos + i] {
                upper @ b'A'..=b'Z' => self.octet.push(upper - b'A' + b'a')?,
                other => self.octet.push(other)?,
            }
        }
        Ok(())
    }
}

impl<'n> std::fmt::Display for TempName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_text(f)
    }
}

#[cfg(test)]
mod test {
    use crate::dnsname::*;

    #[test]
    fn test() -> Result<()> {
        let wire = b"\x05dotat\x02at\x00";
        let mut name = TempName::new();
        name.from_wire(None, wire)?;
        assert_eq!("dotat.at", format!("{}", name));
        Ok(())
    }
}
