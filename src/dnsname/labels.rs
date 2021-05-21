//! Positions of labels on the wire
//! ===============================
//!
//! Sometimes we can't parse a name in one pass; we need to parse
//! the label lengths, then go back to deal with the contents. For
//! instance,
//!
//!   * a HeapName needs to get the length, then allocate, then
//!     copy, to avoid a realloc (and an implicit second copy);
//!
//!   * a TrieName needs to reformat the name in reverse order.
//!
//! The `WireLabels` type is polymorphic, so that we can use `u8`
//! for uncompressed (contiguous) names, which needs less space
//! than `u16` which is necessary for compressed names.
//!
//! `WireLabels` doesn't implement the [`DnsName`] trait, because
//! its lifetime is not coupled to the lifetime of the wire data,
//! which causes safety problems for the methods a `DnsName` needs
//! to provide.

use crate::dnsname::*;
use crate::error::*;
use crate::scratchpad::*;
use std::convert::TryInto;

#[derive(Debug, Default)]
pub struct WireLabels<P> {
    lpos: ScratchPad<P, MAX_LABS>,
    nlen: usize,
}

impl<P> WireLabels<P>
where
    P: Copy,
{
    #[inline(always)]
    pub fn new() -> Self {
        WireLabels { lpos: ScratchPad::new(), nlen: 0 }
    }
}

impl<P> DnsLabels<P> for WireLabels<P> {
    fn labs(&self) -> usize {
        self.lpos.len()
    }

    fn lpos(&self) -> &[P] {
        self.lpos.as_slice()
    }

    fn nlen(&self) -> usize {
        self.nlen
    }
}

// The trait bounds on the generic implementation are a pain,
// and we only need them to be instantiated at two types.

macro_rules! impl_from_wire {
    ($p:ty) => {
        impl FromWire for WireLabels<$p> {
            fn clear(&mut self) {
                self.lpos.clear();
                self.nlen = 0;
            }

            fn from_wire(&mut self, wire: &[u8], pos: usize) -> Result<usize> {
                Dodgy::fun(name_from_wire, self, wire, pos)
            }
        }

        impl LabelFromWire for WireLabels<$p> {
            fn label_from_wire(
                &mut self,
                _: Dodgy,
                pos: usize,
                llen: u8,
            ) -> Result<()> {
                self.lpos.push(pos.try_into().or(Err(Error::WideWire))?)?;
                match self.nlen + 1 + llen as usize {
                    long if long > MAX_NAME => Err(Error::NameLength),
                    short => Ok(self.nlen = short),
                }
            }
        }
    };
}

impl_from_wire!(u8);
impl_from_wire!(u16);
