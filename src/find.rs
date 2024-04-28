use core::simd::prelude::*;
use std::ops::{BitOrAssign, BitXor};

use crate::{INDEXES, LANES, MINUSONES, ZEROES};

#[derive(Debug)]
pub struct SimdFind<'a, const N: usize> {
    needle_lanes: [Simd<u8, LANES>; N],
    haystack: &'a [u8],
    indexes: (Simd<u8, LANES>, usize),
    read: usize,
}

impl<'a, const N: usize> SimdFind<'a, N> {
    pub fn new(needles: [u8; N], haystack: &'a [u8]) -> SimdFind<'a, N> {
        let mut needle_lanes = [Simd::<u8, LANES>::splat(0); N];
        for (i, lane) in needle_lanes.iter_mut().enumerate() {
            *lane = Simd::splat(needles[i]);
        }

        Self {
            needle_lanes,
            haystack,
            indexes: (Simd::splat(u8::MAX), 0),
            read: 0,
        }
    }

    pub fn consume_first_match(&mut self) -> Option<usize> {
        let (indexes, offset) = &mut self.indexes;

        let index = indexes.reduce_min();

        if index == u8::MAX {
            return None;
        }

        let index = index as usize;
        indexes[index] = u8::MAX;

        Some(index + *offset)
    }

    pub fn load_chunk(&mut self) {
        let data = Simd::<u8, LANES>::load_or_default(self.haystack);

        // For each byte we search for we must xor it.
        let mut res_mask = Mask::<i8, LANES>::splat(false);
        for needle in &self.needle_lanes {
            res_mask.bitor_assign(data.bitxor(needle).simd_eq(ZEROES));
        }
        let indexes = res_mask.select(INDEXES, MINUSONES).cast();

        self.indexes = (indexes, self.read);
        self.read += LANES;
    }
}

macro_rules! handle_match {
    ($self:ident) => {
        if let Some(index) = $self.consume_first_match() {
            return Some(index);
        }
    };
}

impl<'a, const N: usize> Iterator for SimdFind<'a, N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        handle_match!(self);

        while self.haystack.len() > LANES {
            self.load_chunk();
            self.haystack = &self.haystack[LANES..];
            handle_match!(self);
        }

        self.load_chunk();
        self.haystack = &[];
        handle_match!(self);

        None
    }
}
