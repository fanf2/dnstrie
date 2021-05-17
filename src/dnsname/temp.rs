//! Temporary copy of a DNS name
//! ============================
//!
//! This kind of name is decompressed and canonicalized to lower case.
//! The name and label pointers are stored in its workpad.

use super::*;
use core::convert::TryInto;

pub struct TempName<'n> {
    pad: &'n TempNameWorkPad,
    label: &'n [u8],
    octet: &'n [u8],
}

pub struct TempNameWorkPad {
    label: WorkPad<u8, MAX_LABEL>,
    octet: WorkPad<u8, MAX_OCTET>,
}

macro_rules! tempname_workpad {
    () => {
        TempNameWorkPad {
            label: workpad_uninit!(u8, MAX_LABEL),
            octet: workpad_uninit!(u8, MAX_OCTET),
        }
    };
}

impl<'n> DnsName for TempName<'n> {
    type NameRef = &'n [u8];

    type WorkPad = &'n mut TempNameWorkPad;

    fn namelen(&self) -> usize {
        self.octet.len()
    }

    fn labels(&self) -> usize {
        self.label.len()
    }

    fn label(&self, lab: usize) -> Option<&'n [u8]> {
        Some(slice_label(self.octet, *self.label.get(lab)? as usize))
    }

    fn parsed_label(
        pad: Self::WorkPad,
        wire: Self::NameRef,
        lpos: usize,
        llen: u8,
    ) -> Result<()> {
        pad.label.push(pad.octet.len().try_into()?);
        pad.octet.push(llen);
        for i in 1..=llen as usize {
            match wire[lpos + i] {
                upper @ b'A'..=b'Z' => pad.octet.push(upper - b'A' + b'a'),
                other => pad.octet.push(other),
            }
        }
        Ok(())
    }

    fn parsed_name(pad: Self::WorkPad) -> Self {
        let label = pad.label.as_slice();
        let octet = pad.octet.as_slice();
        TempName { pad, label, octet }
    }
}
