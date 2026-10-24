use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const DEFAULT_HASH_SEEDS: [u64; 8] = [
    0x9e37_79b9_7f4a_7c15,
    0xc2b2_ae3d_27d4_eb4f,
    0x1656_67b1_9e37_79f9,
    0x85eb_ca77_c2b2_ae63,
    0x27d4_eb2f_1656_67c5,
    0x94d0_49bb_1331_11eb,
    0xd6e8_feb8_6659_fd93,
    0xa5a3_58f1_bbcd_dcab,
];

/// Bitmap-backed Bloom filter.
///
/// Bloom filters are probabilistic: `contains` can return false positives, but
/// it will not return false for an item that was added, as long as the filter is
/// not cleared.
#[derive(Debug, Clone)]
pub struct BitMapBloomFilter {
    bits: Vec<u64>,
    bit_size: usize,
    hash_seeds: Vec<u64>,
}

impl BitMapBloomFilter {
    /// Creates a filter whose bitmap has `size` bits.
    ///
    /// `size` is rounded up internally to whole `u64` words. A zero size is
    /// promoted to one bit so the filter remains usable.
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self::with_hash_count(size, DEFAULT_HASH_SEEDS.len())
    }

    /// Creates a filter with a custom number of hash functions.
    #[must_use]
    pub fn with_hash_count(size: usize, hash_count: usize) -> Self {
        let bit_size = size.max(1);
        let word_count = bit_size.div_ceil(u64::BITS as usize);
        let hash_count = hash_count.max(1);
        let hash_seeds = (0..hash_count)
            .map(|index| {
                DEFAULT_HASH_SEEDS
                    .get(index)
                    .copied()
                    .unwrap_or_else(|| derive_seed(index))
            })
            .collect();

        Self {
            bits: vec![0; word_count],
            bit_size,
            hash_seeds,
        }
    }

    /// Adds a value to the filter.
    pub fn add<T>(&mut self, value: T)
    where
        T: Hash,
    {
        for bit_index in self.bit_indexes(&value) {
            self.set_bit(bit_index);
        }
    }

    /// Returns whether a value may exist in the filter.
    ///
    /// `false` means the value definitely was not added. `true` means the value
    /// may have been added.
    #[must_use]
    pub fn contains<T>(&self, value: T) -> bool
    where
        T: Hash,
    {
        self.bit_indexes(&value)
            .into_iter()
            .all(|bit_index| self.get_bit(bit_index))
    }

    /// Clears all bits.
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    /// Returns the configured bitmap size in bits.
    #[must_use]
    pub fn bit_size(&self) -> usize {
        self.bit_size
    }

    /// Returns the number of hash functions used by this filter.
    #[must_use]
    pub fn hash_count(&self) -> usize {
        self.hash_seeds.len()
    }

    fn bit_indexes<T>(&self, value: &T) -> Vec<usize>
    where
        T: Hash,
    {
        self.hash_seeds
            .iter()
            .map(|seed| {
                let mut hasher = DefaultHasher::new();
                seed.hash(&mut hasher);
                value.hash(&mut hasher);
                (hasher.finish() as usize) % self.bit_size
            })
            .collect()
    }

    fn set_bit(&mut self, bit_index: usize) {
        let word_index = bit_index / u64::BITS as usize;
        let bit_offset = bit_index % u64::BITS as usize;
        self.bits[word_index] |= 1_u64 << bit_offset;
    }

    fn get_bit(&self, bit_index: usize) -> bool {
        let word_index = bit_index / u64::BITS as usize;
        let bit_offset = bit_index % u64::BITS as usize;
        (self.bits[word_index] & (1_u64 << bit_offset)) != 0
    }
}

fn derive_seed(index: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    index.hash(&mut hasher);
    0x517c_c1b7_2722_0a95_u64.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::BitMapBloomFilter;

    #[test]
    fn added_values_are_reported_as_present() {
        let mut filter = BitMapBloomFilter::new(10);

        filter.add("123");
        filter.add("abc");
        filter.add("ddd");

        assert!(filter.contains("123"));
        assert!(filter.contains("abc"));
        assert!(filter.contains("ddd"));
    }

    #[test]
    fn clear_removes_all_known_values() {
        let mut filter = BitMapBloomFilter::new(128);
        filter.add("abc");

        assert!(filter.contains("abc"));

        filter.clear();

        assert!(!filter.contains("abc"));
    }

    #[test]
    fn custom_hash_count_is_supported() {
        let filter = BitMapBloomFilter::with_hash_count(100, 3);

        assert_eq!(filter.bit_size(), 100);
        assert_eq!(filter.hash_count(), 3);
    }
}
