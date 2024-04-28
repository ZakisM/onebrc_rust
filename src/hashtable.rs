// Guide followed at: https://benhoyt.com/writings/hash-table-in-c/

const FNV_OFFSET: usize = 14695981039346656037;
const FNV_PRIME: usize = 1099511628211;

fn hash_key(key: &[u8]) -> usize {
    let mut hash = FNV_OFFSET;

    for &c in key {
        hash ^= usize::from(c);
        hash *= FNV_PRIME;
    }

    hash
}
