use memmap2::Mmap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use std::{fs::File, thread};

use memchr::memchr2_iter;

fn process_chunk(mmap: &Mmap, start: usize, end: usize) {
    let chunk = &mmap[start..end];

    let mut it = memchr2_iter(b';', b'\n', chunk);
    let mut offset = 0;

    loop {
        let (Some(semi_colon), Some(nl)) = (it.next(), it.next()) else {
            break;
        };

        let city = &chunk[offset..semi_colon];
        let temp = &chunk[semi_colon + 1..nl];

        offset = nl + 1;
    }
}

fn main() -> eyre::Result<()> {
    let start_time = std::time::Instant::now();

    let file = File::open("../../IdeaProjects/1brc_typescript/small.txt")?;
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
