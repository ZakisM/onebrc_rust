#![feature(portable_simd, inline_const)]

use core::simd::prelude::*;
use std::{fs::File, thread};

use memmap2::Mmap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

const LANES: usize = 16;
const NEWLINES: Simd<u8, LANES> = Simd::<u8, LANES>::from_array([b'\n'; LANES]);
const ZEROES: Simd<u8, LANES> = Simd::<u8, LANES>::from_array([0; LANES]);
const MINUSONES: Simd<i8, LANES> = Simd::<i8, LANES>::from_array([-1; LANES]);
const INDEXES: Simd<i8, LANES> = Simd::<i8, LANES>::from_array(
    const {
        let mut index = [0; LANES];
        let mut i = 0_usize;
        while i < LANES {
            index[i] = i as i8;
            i += 1;
        }
        index
    },
);

struct MatchedMask(Mask<i8, LANES>);

impl From<Simd<u8, LANES>> for MatchedMask {
    fn from(value: Simd<u8, LANES>) -> Self {
        Self(value.simd_eq(ZEROES))
    }
}

impl MatchedMask {
    pub fn to_indexes(self) -> Simd<u8, LANES> {
        let masked_index = self.0.select(INDEXES, MINUSONES);
        let masked_index: Simd<u8, LANES> = masked_index.cast();

        masked_index
    }
}

#[derive(Debug)]
struct SimdFind<'a> {
    needle: u8,
    haystack: &'a [u8],
    indexes: Simd<u8, LANES>,
    read: usize,
    indexes_offset: usize,
}

impl<'a> SimdFind<'a> {
    pub fn new(needle: u8, haystack: &'a [u8]) -> SimdFind<'a> {
        Self {
            needle,
            haystack,
            indexes: Simd::splat(u8::MAX),
            read: 0,
            indexes_offset: 0,
        }
    }

    pub fn consume_first_match(&mut self) -> Option<usize> {
        let index = self.indexes.reduce_min();

        if index == u8::MAX {
            return None;
        }

        let index = index as usize;
        self.indexes[index] = u8::MAX;

        Some(index + self.indexes_offset)
    }

    pub fn read_chunk(&mut self) {
        let mut data = Simd::<u8, LANES>::load_or_default(self.haystack);

        // For each byte we search for we must xor it.
        data ^= NEWLINES;

        let indexes = MatchedMask::from(data).to_indexes();

        self.indexes = indexes;
        self.indexes_offset = self.read;
        self.read += LANES;
    }
}

impl<'a> Iterator for SimdFind<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.consume_first_match() {
            return Some(index);
        }

        while self.haystack.len() > LANES {
            self.read_chunk();
            self.haystack = &self.haystack[LANES..];

            if let Some(index) = self.consume_first_match() {
                return Some(index);
            }
        }

        self.read_chunk();
        self.haystack = &[];
        if let Some(index) = self.consume_first_match() {
            return Some(index);
        }

        None
    }
}

fn process_chunk(mmap: &Mmap, start: usize, end: usize) {
    let chunk = &mmap[start..end];

    // [1, 2, 3, 4, 5, 6, 7, 8, 10, 14, 15, 16, 17, 18, 19, 20]

    // let mut it = memchr2_iter(b';', b'\n', chunk);
    let mut it = SimdFind::new(b'\n', chunk);
    let mut offset = 0;

    for nl in it {
        // dbg!(nl);
    }
    // while offset + LANES <= chunk.len() {
    //     let mut data = Simd::<u8, LANES>::load_or_default(&chunk[offset..offset + LANES]);
    //     data ^= NEWLINES;
    //     let mask = data.simd_eq(ZEROES);

    //     if let Some(nl) = mask.first_set() {
    //         offset += nl + 1;
    //     } else {
    //         offset += LANES;
    //     }
    // }

    // loop {
    //     let (Some(semi_colon), Some(nl)) = (it.next(), it.next()) else {
    //         break;
    //     };

    //     let city = &chunk[offset..semi_colon];
    //     let temp = &chunk[semi_colon + 1..nl];

    //     offset = nl + 1;
    // }
}

fn main() -> eyre::Result<()> {
    let start_time = std::time::Instant::now();

    let file = File::open("../../IdeaProjects/1brc_typescript/measurements.txt")?;
    let mmap = unsafe { Mmap::map(&file)? };

    let file_size: usize = (file.metadata()?.len()).try_into()?;
    let num_cpus = thread::available_parallelism()?.get();
    let chunk_size = file_size / num_cpus;

    let mut chunk_indexes = Vec::with_capacity(num_cpus);

    let mut start = 0;
    loop {
        let offset = start + chunk_size;

        if offset > file_size {
            chunk_indexes.push((start, file_size));
            break;
        }

        let curr_chunk = &mmap[offset..offset + 100];

        let nl = curr_chunk
            .iter()
            .rposition(|&x| x == 10)
            .expect("Line ending missing in chunk");

        let nl = nl + offset;

        chunk_indexes.push((start, nl));

        start = nl + 1;
    }

    let res = chunk_indexes
        .into_par_iter()
        .map(|(start, end)| process_chunk(&mmap, start, end))
        .collect::<Vec<_>>();

    println!("That took: {}ms", &start_time.elapsed().as_millis());

    Ok(())
}
