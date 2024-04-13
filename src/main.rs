use ahash::AHashMap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::unix::fs::FileExt,
    thread,
};

use memchr::memchr2_iter;

fn process_chunk(start: usize, end: usize) {
    let file = File::open("../../IdeaProjects/1brc_typescript/measurements.txt").unwrap();
    let mut chunk = vec![0; end - start];
    file.read_exact_at(&mut chunk, u64::try_from(start).unwrap())
        .unwrap();

    let mut it = memchr2_iter(b';', b'\n', &chunk);
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

    let mut file = File::open("../../IdeaProjects/1brc_typescript/measurements.txt")?;
    let file_size: usize = (file.metadata()?.len()).try_into()?;
    let num_cpus = thread::available_parallelism()?.get();
    let chunk_size = file_size / num_cpus;

    let mut chunk_indexes = Vec::with_capacity(num_cpus);
    let mut start = 0_usize;

    let mut curr_chunk = Vec::with_capacity(100);
    loop {
        if start + chunk_size > file_size {
            chunk_indexes.push((start, file_size));
            break;
        }

        file.seek(SeekFrom::Current(i64::try_from(chunk_size)?))?;
        // TODO:
        // file.read_exact_at(, )

        file.by_ref().take(100).read_to_end(&mut curr_chunk)?;

        let nl = curr_chunk
            .iter()
            .rposition(|&x| x == 10)
            .expect("Line ending missing in chunk");

        let nl = nl + usize::try_from(file.stream_position()?)?;

        chunk_indexes.push((start, nl));

        start = nl + 1;
        curr_chunk.clear();
        file.seek(SeekFrom::Start(u64::try_from(start)?))?;
    }

    let res = chunk_indexes
        .into_par_iter()
        .map(|(start, end)| process_chunk(start, end))
        .collect::<Vec<_>>();

    println!("That took: {}ms", &start_time.elapsed().as_millis());

    Ok(())
}
