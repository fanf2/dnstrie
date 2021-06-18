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

use crate::prelude::*;

#[derive(Debug, Default)]
pub struct WireLabels<'w, P>
where
    P: Copy,
{
    lpos: ArrayVec<P, MAX_LABS>,
    nlen: usize,
    wire: Option<&'w [u8]>,
}

impl<P> WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    #[inline(always)]
    pub fn new() -> Self {
        WireLabels { lpos: ArrayVec::new(), nlen: 0, wire: None }
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

impl<P> DnsLabels for WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn labs(&self) -> usize {
        self.lpos.len()
    }

    fn nlen(&self) -> usize {
        self.nlen
    }

    fn label(&self, lab: usize) -> Option<&[u8]> {
        let pos = into_usize(*self.lpos.as_slice().get(lab)?);
        let len = *self.wire?.get(pos)? as usize;
        self.wire?.get((pos + 1)..=(pos + len))
    }
}

impl<'n, 'w, P> FromWire<'n, 'w> for WireLabels<'w, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn from_wire(&mut self, wire: &'w [u8], pos: usize) -> Result<usize> {
        let dodgy = Dodgy { bytes: wire };
        self.clear();
        self.wire = Some(wire);
        self.dodgy_from_wire(dodgy, pos).map_err(|err| self.clear_err(err))
    }
}

impl<P> LabelFromWire for WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn label_from_wire(
        &mut self,
        _: Dodgy,
        pos: usize,
        llen: u8,
    ) -> Result<()> {
        self.lpos.try_push(from_usize(pos)?)?;
        self.nlen += 1 + llen as usize;
        if self.nlen > 255 {
            Err(NameLength)
        } else {
            Ok(())
        }
    }
}

impl<P> Eq for WireLabels<'_, P> where P: Copy + TryFrom<usize> + Into<usize> {}

impl<P, Other: DnsLabels> PartialEq<Other> for WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn eq(&self, other: &Other) -> bool {
        cmp_any_names(self, other) == Ordering::Equal
    }
}

impl<P> Ord for WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_any_names(self, other)
    }
}

impl<P, Other: DnsLabels> PartialOrd<Other> for WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn partial_cmp(&self, other: &Other) -> Option<Ordering> {
        Some(cmp_any_names(self, other))
    }
}

impl<P> std::fmt::Display for WireLabels<'_, P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_text(f)
    }
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

fn from_usize<P>(pos: usize) -> Result<P>
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    P::try_from(pos).or(Err(BugWirePos(pos)))
}

fn into_usize<P>(pos: P) -> usize
where
    P: Copy + TryFrom<usize> + Into<usize>,
{
    <P as Into<usize>>::into(pos)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<()> {
        let wire = b"\x05dotat\x02at\x00";
        let mut name = WireLabels::<u8>::new();
        name.from_wire(wire, 0)?;
        assert_eq!("dotat.at", format!("{}", name));
        Ok(())
    }
}
