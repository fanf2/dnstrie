//! BlimpVec, a test oracle for BmpVec
//! ==================================
//!
//! This is a container type that should work the same as BmpVec, but
//! it's simpler: no compression, no unsafe code.

use std::convert::TryInto;

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

    pub fn contains<N>(&self, pos: N) -> bool
    where
        N: TryInto<u8>,
    {
        self.get(pos).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
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
        match self.get_pos(pos) {
            Some(pos) => std::mem::replace(&mut self.vec[pos], val),
            None => panic!("BlimpVec position {:?} out of range", pos),
        }
    }
}
