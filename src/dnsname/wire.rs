//! Positions of labels on the wire
//! ===============================
//!
//! This kind of name has been parsed from a DNS message but it has
//! not been copied. This is for use when we can't parse a name in one
//! pass; we need to parse the label lengths, then go back to deal
//! with the contents.
//!
//! The `WireLabels` type is polymorphic, so that we can use `u8`
//! for uncompressed (contiguous) names, which needs less space
//! than `u16` which is necessary for compressed names.

use crate::dnsname::*;
use crate::scratchpad::*;
use core::cmp::Ordering;
use core::convert::TryInto;

#[derive(Debug, Default)]
pub struct WireLabels<'w, P> {
    lpos: ScratchPad<P, MAX_LABS>,
    nlen: usize,
    wire: Option<&'w [u8]>,
}

impl<'w, P> WireLabels<'w, P>
where
    P: Copy,
{
    #[inline(always)]
    pub fn new() -> Self {
        WireLabels { lpos: ScratchPad::new(), nlen: 0, wire: None }
    }

    pub fn clear(&mut self) {
        self.lpos.clear();
        self.nlen = 0;
        self.wire = None;
    }

    fn clear_err(&mut self, err: Error) -> Error {
        self.clear();
        err
    }
}

macro_rules! impl_wire_labels {
    ($p:ident) => {
        impl<'w> DnsLabels for WireLabels<'w, $p> {
            fn labs(&self) -> usize {
                self.lpos.len()
            }

            fn nlen(&self) -> usize {
                self.nlen
            }

            fn label(&self, lab: usize) -> Option<&[u8]> {
                let pos = *self.lpos.as_slice().get(lab)? as usize;
                let len = *self.wire?.get(pos)? as usize;
                self.wire?.get((pos + 1)..=(pos + len))
            }
        }

        impl<'n, 'w> FromWire<'n, 'w> for WireLabels<'w, $p> {
            fn from_wire(
                &mut self,
                wire: &'w [u8],
                pos: usize,
            ) -> Result<usize> {
                let dodgy = Dodgy { bytes: wire };
                self.clear();
                self.wire = Some(wire);
                self.dodgy_from_wire(dodgy, pos)
                    .map_err(|err| self.clear_err(err))
            }
        }

        impl<'w> LabelFromWire for WireLabels<'w, $p> {
            fn label_from_wire(
                &mut self,
                _: Dodgy,
                pos: usize,
                llen: u8,
            ) -> Result<()> {
                self.lpos.push(pos.try_into()?)?;
                self.nlen += 1 + llen as usize;
                Ok(())
            }
        }

        impl Eq for WireLabels<'_, $p> {}

        impl<Other: DnsLabels> PartialEq<Other> for WireLabels<'_, $p> {
            fn eq(&self, other: &Other) -> bool {
                cmp_any_names(self, other) == Ordering::Equal
            }
        }

        impl Ord for WireLabels<'_, $p> {
            fn cmp(&self, other: &Self) -> Ordering {
                cmp_any_names(self, other)
            }
        }

        impl<Other: DnsLabels> PartialOrd<Other> for WireLabels<'_, $p> {
            fn partial_cmp(&self, other: &Other) -> Option<Ordering> {
                Some(cmp_any_names(self, other))
            }
        }

        impl std::fmt::Display for WireLabels<'_, $p> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.to_text(f)
            }
        }
    };
}

fn cmp_any_labels(aa: &[u8], bb: &[u8]) -> Ordering {
    for chr in 0.. {
        let a = &aa.get(chr).map(|a| a.to_ascii_lowercase());
        let b = &bb.get(chr).map(|b| b.to_ascii_lowercase());
        match a.cmp(b) {
            Ordering::Equal if a.is_none() && b.is_none() => break,
            Ordering::Equal => continue,
            ne => return ne,
        }
    }
    Ordering::Equal
}

fn cmp_any_names<A, B>(aa: &A, bb: &B) -> Ordering
where
    A: DnsLabels,
    B: DnsLabels,
{
    for lab in 0.. {
        match (aa.rlabel(lab), bb.rlabel(lab)) {
            (None, None) => break,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => match cmp_any_labels(a, b) {
                Ordering::Equal => continue,
                ne => return ne,
            },
        }
    }
    Ordering::Equal
}

impl_wire_labels!(u8);
impl_wire_labels!(u16);

#[cfg(test)]
mod test {
    use crate::dnsname::*;

    #[test]
    fn test() -> Result<()> {
        let wire = b"\x05dotat\x02at\x00";
        let mut name = WireLabels::<u8>::new();
        name.from_wire(wire, 0)?;
        assert_eq!("dotat.at", format!("{}", name));
        Ok(())
    }
}
