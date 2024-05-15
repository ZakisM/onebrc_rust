use core::simd::prelude::*;

use crate::{INDEXES, LANES, NULLS};

// TODO: Inline?

#[derive(Debug)]
pub struct SimdFind<'a> {
    needle_lane: Simd<u8, LANES>,
    haystack: &'a [u8],
    indexes: (Simd<u8, LANES>, usize),
    read: usize,
}

impl<'a> SimdFind<'a> {
    pub fn new(needle: u8, haystack: &'a [u8]) -> SimdFind<'a> {
        Self {
            needle_lane: Simd::<u8, LANES>::splat(needle),
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

    pub fn load_chunk(&mut self, slice: &[u8]) {
        let data = Simd::<u8, LANES>::from_slice(slice);

        let res_mask = data.simd_eq(self.needle_lane);
        let indexes = res_mask.select(INDEXES, NULLS);

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

impl<'a> Iterator for SimdFind<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        handle_match!(self);

        while self.haystack.len() > LANES {
            self.load_chunk(&self.haystack[..LANES]);
            self.haystack = &self.haystack[LANES..];
            handle_match!(self);
        }

        let mut remaining = [0; LANES];
        remaining[..self.haystack.len()].copy_from_slice(self.haystack);

        self.load_chunk(&remaining);
        self.haystack = &[];
        handle_match!(self);

        None
    }
}
