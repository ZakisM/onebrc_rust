#![feature(allocator_api, portable_simd)]

use core::simd::prelude::*;
use std::{
    alloc::System,
    fs::File,
    io::{stdout, Write},
    sync::Arc,
    thread,
};

use ahash::{HashMap, HashMapExt};
use bumpalo_herd::Herd;
use crossbeam_channel::Sender;
use fastcmp::Compare;
use find::SimdFind;
use memmap2::Mmap;

mod find;
mod hashtable;

#[global_allocator]
static GLOBAL: System = System;

pub const LANES: usize = 16;
pub const NULLS: Simd<u8, LANES> = Simd::<u8, LANES>::from_array([u8::MAX; LANES]);
pub const INDEXES: Simd<u8, LANES> = Simd::<u8, LANES>::from_array(
    const {
        let mut index = [0_u8; LANES];
        let mut i = 0_usize;
        while i < LANES {
            index[i] = i as u8;
            i += 1;
        }
        index
    },
);
const FNV_OFFSET: usize = 14695981039346656037;
const FNV_PRIME: usize = 1099511628211;
const INITIAL_CAPACITY: usize = 1 << 17;

#[derive(Clone, Copy, Debug, Default)]
struct Stat {
    min: isize,
    max: isize,
    sum: isize,
    count: isize,
}

#[derive(Clone, Debug, Default)]
struct Entry<'a> {
    key: Option<&'a [u8]>,
    value: Stat,
}

#[derive(Debug)]
struct ResultEntry {
    key: Box<[u8]>,
    value: Stat,
}

#[inline(always)]
fn byte_to_digit(byte: u8) -> isize {
    (byte as isize) - (b'0' as isize)
}

#[inline(always)]
fn process_chunk(mmap: &Mmap, start: usize, end: usize, allocator: &Herd, tx: Sender<ResultEntry>) {
    let chunk = &mmap[start..end];

    let mut it = SimdFind::new(b'\n', chunk);
    let mut offset = 0;

    let allocator = allocator.get();
    let mut seen = bumpalo::vec![in allocator.as_bump(); Entry::default(); INITIAL_CAPACITY];

    loop {
        let Some(nl_idx) = it.next() else {
            break;
        };

        let chunk = unsafe { chunk.get_unchecked(offset..nl_idx) };

        let mut city: &[u8] = &[];
        let mut hash = FNV_OFFSET;

        for (i, &b) in chunk.iter().enumerate() {
            if b == b';' {
                city = unsafe { chunk.get_unchecked(..i) };
                break;
            }
            hash ^= usize::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        let mut index = hash & (INITIAL_CAPACITY - 1);

        let temp_parsed = match unsafe { chunk.get_unchecked(city.len() + 1..) } {
            // -9.9
            [b'-', t, b'.', d] => -(((byte_to_digit(*t)) * 10) + (byte_to_digit(*d))),
            // 99.9
            [h, t, b'.', d] => {
                ((byte_to_digit(*h)) * 100) + ((byte_to_digit(*t)) * 10) + (byte_to_digit(*d))
            }
            // 9.9
            [t, b'.', d] => ((byte_to_digit(*t)) * 10) + (byte_to_digit(*d)),
            // -99.9
            [b'-', h, t, b'.', d] => {
                -(((byte_to_digit(*h)) * 100) + ((byte_to_digit(*t)) * 10) + (byte_to_digit(*d)))
            }
            _ => unreachable!(),
        };

        loop {
            let entry = unsafe { seen.get_unchecked_mut(index) };

            match entry.key {
                None => {
                    entry.key = Some(city);
                    entry.value = Stat {
                        min: temp_parsed,
                        max: temp_parsed,
                        sum: temp_parsed,
                        count: 1,
                    };
                    break;
                }
                // Using feq here is 10% faster than using _platform_memcmp
                Some(existing) if existing.feq(city) => {
                    entry.value = Stat {
                        min: std::cmp::min(entry.value.min, temp_parsed),
                        max: std::cmp::max(entry.value.max, temp_parsed),
                        sum: entry.value.sum + temp_parsed,
                        count: entry.value.count + 1,
                    };
                    break;
                }
                _ => {
                    index += 1;
                    if index >= INITIAL_CAPACITY {
                        index = 0;
                    }
                }
            }
        }

        offset = nl_idx + 1;
    }

    for item in seen {
        if let Some(key) = item.key {
            let res = ResultEntry {
                key: Box::from(key),
                value: item.value,
            };

            tx.send(res).expect("Failed to send to result channel");
        }
    }
}

fn main() -> eyre::Result<()> {
    let herd = Arc::new(Herd::new());
    let allocator_member = herd.get();
    let allocator = allocator_member.as_bump();

    let file = File::open("measurements.txt")?;
    let mmap = Arc::new(unsafe { Mmap::map(&file)? });

    let file_size: usize = file.metadata()?.len().try_into()?;
    let num_cpus = thread::available_parallelism()?.get();
    let chunk_size = file_size / num_cpus;

    let mut chunk_indexes = Vec::with_capacity_in(num_cpus, allocator);

    let mut start = 0;
    loop {
        let offset = start + chunk_size;

        if offset > file_size {
            chunk_indexes.push((start, file_size));
            break;
        }

        let curr_chunk = unsafe { mmap.get_unchecked(offset..offset + 100) };

        let nl = curr_chunk
            .iter()
            .rposition(|&x| x == b'\n')
            .expect("Line ending missing in chunk");

        let nl = nl + offset;

        chunk_indexes.push((start, nl));

        start = nl + 1;
    }

    let (tx, tr) = crossbeam_channel::unbounded();
    for (start, end) in chunk_indexes {
        let mmap = Arc::clone(&mmap);
        let herd = Arc::clone(&herd);
        let tx = tx.clone();

        thread::Builder::new()
            .stack_size(256 << 10)
            .spawn(move || process_chunk(&mmap, start, end, &herd, tx))
            .expect("Failed to spawn thread");
    }
    drop(tx);

    let mut all_results: HashMap<Box<[u8]>, Stat> = HashMap::with_capacity(413);

    while let Ok(thread_entry) = tr.recv() {
        all_results
            .entry(thread_entry.key)
            .and_modify(|entry: &mut Stat| {
                *entry = Stat {
                    min: std::cmp::min(entry.min, thread_entry.value.min),
                    max: std::cmp::max(entry.max, thread_entry.value.max),
                    sum: entry.sum + thread_entry.value.sum,
                    count: entry.count + thread_entry.value.count,
                };
            })
            .or_insert(thread_entry.value);
    }

    let mut keys = all_results.keys().collect::<Vec<_>>();
    keys.sort();

    let mut stdout = stdout().lock();

    stdout.write_all(b"{")?;

    for (i, city) in keys.into_iter().enumerate() {
        let temp = all_results.get(city).unwrap();
        let min = temp.min as f32 / 10.0;
        let mean = (temp.sum as f32 / temp.count as f32) / 10.0;
        let max = temp.max as f32 / 10.0;

        stdout.write_fmt(format_args!(
            "{}={:.1}/{:.1}/{:.1}",
            unsafe { std::str::from_utf8_unchecked(city) },
            min,
            mean,
            max,
        ))?;

        if i != all_results.len() - 1 {
            stdout.write_all(b", ")?;
        }
    }

    stdout.write_all(b"}")?;

    stdout.flush()?;

    Ok(())
}
