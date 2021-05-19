//! Temporary copy of a DNS name
//! ============================
//!
//! This kind of name is decompressed and canonicalized to lower case.
//! The name and label pointers are stored in its scratch pad.

use crate::dnsname::*;
use crate::error::*;
use crate::scratchpad::*;
use core::convert::TryInto;

#[derive(Debug, Default)]
pub struct ScratchName {
    lpos: ScratchPad<u8, MAX_LABS>,
    name: ScratchPad<u8, MAX_NAME>,
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
}

impl DnsName for ScratchName {
    fn namelen(&self) -> usize {
        self.name.len()
    }

    fn labels(&self) -> usize {
        self.lpos.len()
    }

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let pos = *self.lpos.as_slice().get(lab)?;
        Some(slice_label(self.name.as_slice(), pos as usize))
    }
}

impl FromWire for ScratchName {
    fn parsed_label(
        &mut self,
        wire: Wire,
        rpos: usize,
        llen: u8,
    ) -> Result<()> {
        let wpos = self.name.len().try_into()?;
        self.lpos.push(wpos)?;
        self.name.push(llen)?;
        for i in 1..=llen as usize {
            match wire.get(rpos + i)? {
                upper @ b'A'..=b'Z' => self.name.push(upper - b'A' + b'a')?,
                other => self.name.push(other)?,
            }
        }
        Ok(())
    }
}

impl<'n> std::fmt::Display for ScratchName {
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
        let mut name = ScratchName::new();
        name.from_wire(wire, 0)?;
        assert_eq!("dotat.at", format!("{}", name));
        Ok(())
    }
}
