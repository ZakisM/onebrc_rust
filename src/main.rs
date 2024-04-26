#![feature(portable_simd)]

use core::simd::prelude::*;
use std::{
    collections::HashMap,
    fs::File,
    ops::{BitOrAssign, BitXor},
    thread,
};

use ahash::{AHashMap, AHashSet};
use memmap2::Mmap;
use mimalloc::MiMalloc;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const LANES: usize = 32;
const SEMIS: Simd<u8, LANES> = Simd::<u8, LANES>::from_array([b';'; LANES]);
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

#[derive(Debug)]
struct SimdFind<'a, const N: usize> {
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

const PRIMES: [u64; 26] = [
    15331, 15349, 15359, 15361, 15373, 15377, 15383, 15391, 15401, 15413, 15427, 15439, 15443,
    15451, 15461, 15467, 15473, 15493, 15497, 15511, 15527, 15541, 15551, 15559, 15569, 15581,
];

struct SimpleHasher {
    state: u64,
}
struct BuildSimpleHasher;

impl std::hash::Hasher for SimpleHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            self.state += (b as u64) * PRIMES[i];
        }
    }
}

impl std::hash::BuildHasher for BuildSimpleHasher {
    type Hasher = SimpleHasher;

    fn build_hasher(&self) -> Self::Hasher {
        SimpleHasher { state: PRIMES[0] }
    }
}

// fn hash_city(city: &[u8]) -> usize {
//     assert!(city.len() >= 3);

//     let mut res = PRIMES[0];

//     // TODO: SIMD hash?
//     for (i, &b) in city.iter().enumerate() {
//         res += (b as usize) * PRIMES[i];
//     }

//     res
// }

fn process_chunk(mmap: &Mmap, start: usize, end: usize) {
    let chunk = &mmap[start..end];

    let mut it = SimdFind::new([b'\n'], chunk);
    let mut offset = 0;

    let mut seen: AHashMap<&[u8], &[u8]> = AHashMap::with_capacity(413);
    // let mut seen = HashMap::with_capacity_and_hasher(413, BuildSimpleHasher);

    loop {
        let Some(nl_idx) = it.next() else {
            break;
        };

        let chunk = &chunk[offset..nl_idx];
        // Read the last 7 bytes of the end of the chunk, because semi colon is at the end,
        // chunk is at minimum 7 bytes long and temp is 5 bytes at most.
        let chunk_end = chunk.len() - 7;
        let semi_colon_idxs: Simd<u8, LANES> = Simd::load_or_default(&chunk[chunk_end..])
            .bitxor(SEMIS)
            .simd_eq(ZEROES)
            .select(INDEXES, MINUSONES)
            .cast();
        let semi_colon_idx = semi_colon_idxs.reduce_min() as usize + chunk_end;

        let city = &chunk[..semi_colon_idx];
        let temp = &chunk[semi_colon_idx + 1..];

        // let city_hash = hash_city(city);
        // dbg!(&city_hash % 412);
        // if let Some(&existing) = seen.get(&city_hash) {
        //     if existing != city {
        //         dbg!(&seen.len());
        //         // dbg!(std::str::from_utf8(city));
        //         // dbg!(std::str::from_utf8(existing));
        //         // dbg!(city_hash);
        //         // dbg!(hash_city(existing));
        //         panic!("Collision found for {city:?}[{existing:?}][{city_hash}]");
        //     }
        // }
        // seen.insert(city, temp);
        seen.entry(city).and_modify(|e| *e = temp).or_insert(temp);

        offset = nl_idx + 1;
    }
}

fn main() -> eyre::Result<()> {
    let start_time = std::time::Instant::now();

    let file = File::open("small.txt")?;
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
    // for (start, end) in chunk_indexes {
    //     process_chunk(&mmap, start, end);
    // }

    println!("That took: {}ms", &start_time.elapsed().as_millis());

    Ok(())
}
