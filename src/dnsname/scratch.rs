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
}

impl DnsLabels<u8> for ScratchName {
    fn labs(&self) -> usize {
        self.lpos.len()
    }

    fn lpos(&self) -> &[u8] {
        self.lpos.as_slice()
    }

    fn nlen(&self) -> usize {
        self.name.len()
    }
}

impl DnsName for ScratchName {
    fn name(&self) -> &[u8] {
        self.name.as_slice()
    }

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let pos = *self.lpos().get(lab)?;
        Some(slice_label(self.name(), pos as usize))
    }
}

impl std::fmt::Display for ScratchName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_text(f)
    }
}

impl FromWire for ScratchName {
    fn clear(&mut self) {
        self.lpos.clear();
        self.name.clear();
    }

    fn from_wire(&mut self, wire: &[u8], pos: usize) -> Result<usize> {
        Dodgy::fun(name_from_wire, self, wire, pos)
    }
}

impl LabelFromWire for ScratchName {
    fn label_from_wire(
        &mut self,
        wire: Dodgy,
        rpos: usize,
        llen: u8,
    ) -> Result<()> {
        let wpos = self.name.len().try_into().or(Err(NameLength))?;
        self.lpos.push(wpos)?;
        self.name.push(llen)?;
        for i in 1..=llen as usize {
            self.name.push(wire.get(rpos + i)?.to_ascii_lowercase())?;
        }
        Ok(())
    }
}

impl FromText for ScratchName {
    fn from_text(&mut self, text: &[u8]) -> Result<usize> {
        Dodgy::fun(name_from_wire, self, text, 0)
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
