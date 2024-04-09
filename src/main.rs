use ahash::AHashMap;
use std::{
    fs::File,
    io::{BufReader, Read},
};

use memchr::memchr2_iter;

const CHUNK_SIZE: usize = 256 * (1 << 10); // 256kb

// TODO: See how much faster memchr is
fn process_chunk(chunk: &[u8]) -> AHashMap<&str, &str> {
    // Safety: Should re-check this if we change CHUNK_SIZE
    let mut chunk = unsafe { std::str::from_utf8_unchecked(chunk) };

    let mut res = AHashMap::with_capacity(413);

    loop {
        let mut it = memchr2_iter(b';', b'\n', chunk.as_bytes());
        let (Some(semi_colon), Some(nl)) = (it.next(), it.next()) else {
            break;
        };

        let city = &chunk[..semi_colon];
        let temp = &chunk[semi_colon + 1..nl];

        res.insert(city, temp);

        chunk = &chunk[nl + 1..];
    }

    res
}

fn main() -> eyre::Result<()> {
    let start = std::time::Instant::now();

    let file = File::open("../../IdeaProjects/1brc_typescript/measurements.txt")?;
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, file);

    let mut line = Vec::with_capacity(CHUNK_SIZE + 31);

    while let Ok(read) = reader
        .by_ref()
        .take(CHUNK_SIZE as u64)
        .read_to_end(&mut line)
    {
        if read == 0 {
            break;
        }

        let nl = line
            .iter()
            .rposition(|&x| x == 10)
            .expect("Line ending missing in chunk");

        process_chunk(&line[..nl]);

        line.copy_within(nl + 1.., 0);
        line.truncate(line.len() - nl - 1);
    }

    println!("That took: {}ms", &start.elapsed().as_millis());

    Ok(())
}
