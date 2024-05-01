// Guide followed at: https://benhoyt.com/writings/hash-table-in-c/

const FNV_OFFSET: usize = 14695981039346656037;
const FNV_PRIME: usize = 1099511628211;
const INITIAL_CAPACITY: usize = 1 << 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Entry<'a, T: Clone + Copy> {
    key: &'a [u8],
    value: T,
}

#[derive(Debug, Eq, PartialEq)]
pub struct HashTable<'a, T: Clone + Copy> {
    entries: [Option<Entry<'a, T>>; INITIAL_CAPACITY],
    capacity: usize,
    length: usize,
}

impl<'a, T: Clone + Copy> HashTable<'a, T> {
    pub fn new() -> Self {
        Self {
            entries: [None; INITIAL_CAPACITY],
            capacity: INITIAL_CAPACITY,
            length: 0,
        }
    }

    #[inline(always)]
    fn hash_key(key: &[u8]) -> usize {
        let mut hash = FNV_OFFSET;

        for &c in key {
            hash ^= usize::from(c);
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        hash
    }

    #[inline(always)]
    pub fn get(&self, key: &[u8]) -> Option<&T> {
        let hash = Self::hash_key(key);
        let mut index = hash & (self.capacity - 1);

        // Required?
        assert!(index < self.entries.len());

        while let Some(entry) = &self.entries[index] {
            if key == entry.key {
                return Some(&entry.value);
            }
            index = index.wrapping_add(1);
        }

        None
    }

    #[inline(always)]
    pub fn set(&mut self, key: &'a [u8], value: T) {
        // if self.length >= self.capacity / 2 {
        //     let new_capacity = self.capacity * 2;

        //     self.entries.resize(new_capacity, None);
        //     self.capacity = new_capacity;
        // }

        self.set_entry(key, value);
    }

    #[inline(always)]
    fn set_entry(&mut self, key: &'a [u8], value: T) -> Option<&[u8]> {
        let hash = Self::hash_key(key);
        let mut index = hash & (self.capacity - 1);

        // Required?
        assert!(index < self.entries.len());

        // [0, 0, 0, 0, 0, 0, 0, 0]
        // Insert a -> index = 2
        // [0, 0, 1, 0, 0, 0, 0, 0]
        // Insert b
        // But there is a collision as -> index = 2
        // [0, 0, 1, 0, 0, 0, 0, 0]

        // TODO: This is the issue, either SIMD or optimize some other way
        while let Some(entry) = &mut self.entries[index] {
            if key == entry.key {
                entry.value = value;
                return Some(entry.key);
            }
            index = index.wrapping_add(1);
        }

        // If didn't find the key, insert it
        // let curr = &mut self.entries[index];
        // *curr = Some(Entry { key, value });
        // self.length += 1;

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_create_hashtable() {
    //     assert_eq!(
    //         HashTable::new(),
    //         HashTable::<&[u8]> {
    //             entries: vec![None; 16],
    //             capacity: 16,
    //             length: 0,
    //         }
    //     );
    // }

    // #[test]
    // fn test_modify_hashtable() {
    //     let mut table = HashTable::<&[u8]>::new();
    //     table.set(&[0, 1, 2], &[3, 4, 5]);

    //     assert_eq!(table.length, 1);
    //     assert_eq!(table.capacity, 16);
    //     assert_eq!(table.get(&[0, 1, 2]), Some(&[3, 4, 5].as_slice()));
    // }

    #[test]
    fn test_modify_hashtable_multiple() {
        let mut table = HashTable::<&[u8]>::new();
        table.set(&[0], &[0, 0, 0]);
        table.set(&[1], &[0, 0, 1]);
        table.set(&[2], &[0, 0, 2]);
        table.set(&[3], &[0, 0, 3]);
        table.set(&[4], &[0, 0, 4]);
        table.set(&[5], &[0, 0, 5]);
        table.set(&[6], &[0, 0, 6]);
        table.set(&[7], &[0, 0, 7]);
        // dbg!(&table);
        assert_eq!(table.get(&[0]), Some(&[0, 0, 0].as_slice()));
        table.set(&[8], &[0, 0, 8]);
        assert_eq!(table.get(&[0]), Some(&[0, 0, 0].as_slice()));
        // dbg!(&table);
        // assert_eq!(table.get(&[0]), Some(&[0, 0, 0].as_slice()));

        // assert_eq!(table.length, 9);
        // assert_eq!(table.capacity, 32);
        // assert_eq!(table.get(&[0]), Some(&[0, 0, 0].as_slice()));
    }

    // #[test]
    // fn test_modulo() {
    //     //         10            8            2
    //     assert_eq!(0b0000_1010 % 0b0000_1000, 0b0000_0010);
    //     assert_eq!(0b0000_1010 & 0b0000_0111, 0b0000_0010);

    //     //         128           8            0
    //     assert_eq!(0b1000_0000 % 0b0000_1000, 0b0000_0000);
    //     //         128           8            0
    //     assert_eq!(0b1000_0000 & 0b0000_0111, 0b0000_0000);
    // }
}
