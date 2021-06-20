#![allow(dead_code)]

use crate::prelude::*;
use core::mem::ManuallyDrop;

pub struct DnsTrie<T> {
    len: usize,
    root: Twig<T>,
}

impl<T> Default for DnsTrie<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DnsTrie<T> {
    pub fn new() -> Self {
        DnsTrie { len: 0, root: Twig::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(unused_variables)]
    pub fn insert<'n, N>(&mut self, name: &'n N, val: T) -> Option<T>
    where
        N: DnsLabels,
        HeapName: From<&'n N>,
    {
        let leaf = Twig::leaf_from(HeapName::from(name), val);
        if self.len == 0 {
            self.root = leaf;
            self.len = 1;
            return None;
        }

        let mut key = TrieName::new();
        key.from_dns_name(name);

        unimplemented!();
    }
}

union TwigData<T> {
    element: ManuallyDrop<T>,
    twigmut: *mut Twig<T>,
    twigref: *const Twig<T>,
}

struct Twig<T> {
    meta: u64,
    data: TwigData<T>,
}

impl<T> Default for Twig<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Twig<T> {
    fn new() -> Self {
        // SAFETY: we are responsible for dropping the empty BmpVec.
        let (_, twigs) = unsafe { BmpVec::new().into_raw_parts() };
        Twig { meta: 0, data: TwigData { twigmut: twigs } }
    }

    fn leaf_from(key: HeapName, val: T) -> Self {
        // SAFETY: we are responsible for dropping the key and value.
        let meta = unsafe { key.into_ptr() as u64 };
        let data = TwigData { element: ManuallyDrop::new(val) };
        Twig { meta, data }
    }
}
