use ahash::AHashMap;

use std::{
    fs::File,
    io::{BufReader, Read},
    thread,
};

use memchr::memchr2_iter;

fn process_chunk(chunk: Vec<u8>) {
    // let mut res = AHashMap::with_capacity(448);

    let mut it = memchr2_iter(b';', b'\n', &chunk);
    let mut offset = 0;

    loop {
        let (Some(semi_colon), Some(nl)) = (it.next(), it.next()) else {
            break;
        };

        let city = &chunk[offset..semi_colon];
        let temp = &chunk[semi_colon + 1..nl];

        // res.insert(city, temp);

        offset = nl + 1;
    }
}

fn main() -> eyre::Result<()> {
    let start = std::time::Instant::now();

    let file = File::open("../../IdeaProjects/1brc_typescript/measurements.txt")?;
    let file_size = file.metadata()?.len();
    let num_cpus = thread::available_parallelism()?.get();
    let chunk_size = file_size as usize / num_cpus;

    let mut reader = BufReader::with_capacity(chunk_size, file);

    let mut line = Vec::with_capacity(chunk_size + 31);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus)
        .build()?;
    // let mut handles = Vec::with_capacity(12);

    while let Ok(read) = reader
        .by_ref()
        .take(chunk_size as u64)
        .read_to_end(&mut line)
    {
        if read == 0 {
            break;
        }

        let nl = line
            .iter()
            .rposition(|&x| x == 10)
            .expect("Line ending missing in chunk");

        // Safety: Should re-check this if we change CHUNK_SIZE
        // let line_string = unsafe { String::from_utf8_unchecked(line.clone()) };

        let line_thread = line.clone();
        pool.spawn(|| {
            process_chunk(line_thread);
        });

        line.copy_within(nl + 1.., 0);
        line.truncate(line.len() - nl - 1);
    }

    println!("That took: {}ms", &start.elapsed().as_millis());

    Ok(())
}
