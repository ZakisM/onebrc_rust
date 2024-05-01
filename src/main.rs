#![feature(portable_simd)]

use core::simd::prelude::*;
use std::{fs::File, ops::BitXor, sync::Arc, thread};

use find::SimdFind;
use memmap2::Mmap;
use mimalloc::MiMalloc;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

mod find;
mod hashtable;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub const LANES: usize = 32;
pub const SEMIS: Simd<u8, LANES> = Simd::<u8, LANES>::from_array([b';'; LANES]);
pub const ZEROES: Simd<u8, LANES> = Simd::<u8, LANES>::from_array([0; LANES]);
pub const MINUSONES: Simd<i8, LANES> = Simd::<i8, LANES>::from_array([-1; LANES]);
pub const INDEXES: Simd<i8, LANES> = Simd::<i8, LANES>::from_array(
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
const FNV_OFFSET: usize = 14695981039346656037;
const FNV_PRIME: usize = 1099511628211;
const INITIAL_CAPACITY: usize = 1 << 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Entry<'a, T: Clone + Copy> {
    key: &'a [u8],
    value: T,
}

fn process_chunk(mmap: &Mmap, start: usize, end: usize) {
    let chunk = &mmap[start..end];

    let mut it = SimdFind::new([b'\n'], chunk);
    let mut offset = 0;

    // let mut seen: AHashMap<&[u8], &[u8]> = AHashMap::with_capacity(413);
    // let mut seen = HashMap::with_capacity_and_hasher(413, BuildSimpleHasher);

    let mut seen: [Option<Entry<usize>>; INITIAL_CAPACITY] = [None; INITIAL_CAPACITY];

    loop {
        let Some(nl_idx) = it.next() else {
            break;
        };

        let chunk = unsafe { chunk.get_unchecked(offset..nl_idx) };

        let mut city: &[u8] = &[];
        // let mut temp: &[u8] = &[];

        let mut hash = FNV_OFFSET;

        for (i, b) in chunk.iter().enumerate() {
            if b == &b';' {
                city = unsafe { chunk.get_unchecked(..i) };
                // temp = unsafe { chunk.get_unchecked(i + 1..) };
                break;
            }
            hash ^= usize::from(*b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        let mut index = hash & (INITIAL_CAPACITY - 1);

        loop {
            let entry = unsafe { seen.get_unchecked_mut(index) };

            match entry {
                None => {
                    *entry = Some(Entry {
                        key: city,
                        value: 0,
                    });
                    break;
                }
                Some(existing) if existing.key == city => {
                    existing.value += 1;
                    break;
                }
                _ => (),
            }

            index += 1;
            if index >= INITIAL_CAPACITY {
                index = 0;
            }
        }

        // dbg!(std::str::from_utf8(city));
        // // Read the last 7 bytes of the end of the chunk, because semi colon is at the end,
        // // chunk is at minimum 7 bytes long and temp is 5 bytes at most.
        // let chunk_end = chunk.len() - 7;
        // let semi_colon_idxs: Simd<u8, LANES> = Simd::load_or_default(&chunk[chunk_end..])
        //     .bitxor(SEMIS)
        //     .simd_eq(ZEROES)
        //     .select(INDEXES, MINUSONES)
        //     .cast();
        // let semi_colon_idx = semi_colon_idxs.reduce_min() as usize + chunk_end;

        // assert!(semi_colon_idx < chunk.len());

        // let city = &chunk[..semi_colon_idx];
        // let temp = &chunk[semi_colon_idx + 1..];

        // let mut bytes_iter = chunk.iter().enumerate();
        // let mut key: &[u8] = &[];
        // let mut hash = FNV_OFFSET;

        // while let Some((i, &b)) = bytes_iter.next() {
        //     if b == b';' {
        //         key = &chunk[..i];
        //         break;
        //     }
        //     hash ^= usize::from(b);
        //     hash = hash.wrapping_mul(FNV_PRIME);
        // }

        // let mut index = hash & (INITIAL_CAPACITY - 1);

        // let mut should_insert = true;

        // while let Some(entry) = &mut seen[index] {
        //     if key == entry.key {
        //         // entry.value = value;
        //         entry.value = 0;
        //         should_insert = false;
        //         break;
        //     }

        //     index = index.wrapping_add(1);
        // }

        // if should_insert {
        //     let curr = &mut seen[index];
        //     *curr = Some(Entry { key, value: 0 })
        // }

        // while let Some(entry) = &mut seen[index] {
        //     if
        // }

        // seen.set(city, 0);

        // let city_hash = hashtable::hash_key(city);
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
        // seen.entry(city).and_modify(|e| *e = temp).or_insert(temp);

        offset = nl_idx + 1;
    }
}

fn main() -> eyre::Result<()> {
    let start_time = std::time::Instant::now();

    let file = File::open("small.txt")?;
    let mmap = Arc::new(unsafe { Mmap::map(&file)? });

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

    // let res = chunk_indexes
    //     .into_par_iter()
    //     .map(|(start, end)| process_chunk(&mmap, start, end))
    //     .collect::<Vec<_>>();
    // let mut handles = Vec::with_capacity(num_cpus);

    // for (start, end) in &chunk_indexes {
    // handles.push(std::thread::spawn(process_chunk(&mmap, *start, *end)));
    // }

    std::thread::scope(|s| {
        for (start, end) in chunk_indexes {
            let mmap = Arc::clone(&mmap);

            std::thread::Builder::new()
                // .stack_size(8 * 1024 * 1024)
                .spawn_scoped(s, move || process_chunk(&mmap, start, end))
                .unwrap();
        }
    });

    println!("That took: {}ms", &start_time.elapsed().as_millis());

    Ok(())
}
