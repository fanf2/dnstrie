//! BlimpVec, a test oracle for BmpVec
//! ==================================
//!
//! This is a container type that should work the same as BmpVec,
//! but it's simpler: no compression, no unsafe code.

use std::convert::TryInto;

#[derive(PartialEq)]
pub struct BlimpVec<T> {
    len: usize,
    vec: Vec<Option<T>>,
}

impl<T> BlimpVec<T> {
    pub fn new() -> BlimpVec<T> {
        let mut vec = Vec::new();
        for _ in 0..=63 {
            vec.push(None);
        }
        BlimpVec { len: 0, vec }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, &T)> {
        self.vec
            .iter()
            .enumerate()
            .filter(|(_, elem)| elem.is_some())
            .map(|(pos, elem)| (pos as u8, elem.as_ref().unwrap()))
    }

    pub fn keys(&self) -> impl Iterator<Item = u8> + '_ {
        self.vec
            .iter()
            .enumerate()
            .filter(|(_, elem)| elem.is_some())
            .map(|(pos, _)| pos as u8)
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.vec
            .iter()
            .filter(|elem| elem.is_some())
            .map(|elem| elem.as_ref().unwrap())
    }

    pub fn contains<N>(&self, pos: N) -> bool
    where
        N: TryInto<u8>,
    {
        self.get(pos).is_some()
    }

    fn get_pos<N>(&self, pos: N) -> Option<usize>
    where
        N: TryInto<u8>,
    {
        pos.try_into()
            .ok()
            .map(|pos| pos as usize)
            .filter(|&pos| pos < self.vec.len())
    }

    pub fn get<N>(&self, pos: N) -> Option<&T>
    where
        N: TryInto<u8>,
    {
        self.get_pos(pos).and_then(|pos| self.vec[pos].as_ref())
    }

    pub fn get_mut<N>(&mut self, pos: N) -> Option<&mut T>
    where
        N: TryInto<u8>,
    {
        match self.get_pos(pos) {
            Some(pos) => self.vec[pos].as_mut(),
            _ => None,
        }
    }

    pub fn insert<N>(&mut self, pos: N, val: T) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        self.set(pos, Some(val))
    }

    pub fn remove<N>(&mut self, pos: N) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        self.set(pos, None)
    }

    pub fn set<N>(&mut self, pos: N, val: Option<T>) -> Option<T>
    where
        N: TryInto<u8> + Copy + std::fmt::Debug,
    {
        let somenew = val.as_ref().is_some();
        let old = match self.get_pos(pos) {
            Some(pos) => std::mem::replace(&mut self.vec[pos], val),
            None => panic!("BlimpVec position {:?} out of range", pos),
        };
        let someold = old.as_ref().is_some();
        match (someold, somenew) {
            (false, true) => self.len += 1,
            (true, false) => self.len -= 1,
            _ => (),
        }
        old
    }
}

impl<T> std::fmt::Debug for BlimpVec<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlimpVec")?;
        f.debug_map().entries(self.iter()).finish()
    }
}

use crate::bmpvec::*;

impl<T> From<&BlimpVec<T>> for BmpVec<T>
where
    T: Copy,
{
    fn from(blimp: &BlimpVec<T>) -> BmpVec<T> {
        let mut bmp = BmpVec::new();
        for (pos, val) in blimp.iter() {
            bmp.insert(pos, *val);
        }
        bmp
    }
}

impl<T> From<&BmpVec<T>> for BlimpVec<T>
where
    T: Copy,
{
    fn from(bmp: &BmpVec<T>) -> BlimpVec<T> {
        let mut blimp = BlimpVec::new();
        for (pos, val) in bmp.iter() {
            blimp.insert(pos, *val);
        }
        blimp
    }
}
