use std::cmp::Ordering;

use super::key_value::KeyValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValues<T> {
    sorted_set: Vec<T>,
}

impl<T> KeyValues<T>
where
    T: AsRef<dyn KeyValue>,
    // T: std::cmp::PartialEq,
{
    /// Creates an empty KeyValues instance
    pub fn empty() -> Self {
        KeyValues {
            sorted_set: Vec::with_capacity(0),
        }
    }

    // /// Creates a KeyValues from a single key-value pair
    // pub fn from_kv(key: String, value: String) -> Self {
    //     KeyValues {
    //         sorted_set: vec![KeyValue::new(key, value)],
    //     }
    // }

    // /// Creates a KeyValues from an array of key-value pairs
    // pub fn from_array(kvs: &[T]) -> Self {
    //     let mut sorted = kvs.to_vec();
    //     sorted.sort();
    //     sorted.dedup_by(|a, b| a.key() == b.key());
    //     KeyValues { sorted_set: sorted }
    // }

    /// Creates a KeyValues from key-value pairs in a vector
    // pub fn from_vec(kvs: Vec<T>) -> Self {
    //     let mut sorted = kvs;
    //     sorted.sort();
    //     sorted.dedup_by(|a, b| a.key() == b.key());
    //     KeyValues { sorted_set: sorted }
    // }

    // /// Creates a KeyValues from an iterator of key-value pairs
    // pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    //     let mut sorted: Vec<T> = iter.into_iter().collect();
    //     sorted.sort();
    //     sorted.dedup_by(|a, b| a.key() == b.key());
    //     KeyValues { sorted_set: sorted }
    // }

    // /// Creates a KeyValues from string pairs (key1, value1, key2, value2, ...)
    // pub fn from_str_pairs(pairs: &[&str]) -> Result<Self, &'static str> {
    //     if pairs.len() % 2 != 0 {
    //         return Err("Number of elements must be even (key-value pairs)");
    //     }

    //     let kvs: Vec<T> = pairs
    //         .chunks(2)
    //         .map(|chunk| T::new(chunk[0].to_string(), chunk[1].to_string()))
    //         .collect();

    //     Ok(Self::from_vec(kvs))
    // }

    /// Merges this KeyValues with another
    pub fn and(self, other: Self) -> Self {
        if other.sorted_set.is_empty() {
            return self;
        }

        // if self == other {
        //     return self;
        // }

        if self.sorted_set.is_empty() {
            return other;
        }

        let mut merged = Vec::with_capacity(self.sorted_set.len() + other.sorted_set.len());
        let mut self_iter = self.sorted_set.into_iter();
        let mut other_iter = other.sorted_set.into_iter();

        let mut self_next = self_iter.next();
        let mut other_next = other_iter.next();

        loop {
            match (self_next.take(), other_next.take()) {
                (Some(self_kv), Some(other_kv)) => {
                    match self_kv.as_ref().key().cmp(&other_kv.as_ref().key()) {
                        Ordering::Less => {
                            merged.push(self_kv);
                            self_next = self_iter.next();
                            other_next = Some(other_kv);
                        }
                        Ordering::Greater => {
                            merged.push(other_kv);
                            self_next = Some(self_kv);
                            other_next = other_iter.next();
                        }
                        Ordering::Equal => {
                            // On conflict, prefer the other's value
                            merged.push(other_kv);
                            self_next = self_iter.next();
                            other_next = other_iter.next();
                        }
                    }
                }
                (Some(self_kv), None) => {
                    merged.push(self_kv);
                    merged.extend(self_iter);
                    break;
                }
                (None, Some(other_kv)) => {
                    merged.push(other_kv);
                    merged.extend(other_iter);
                    break;
                }
                (None, None) => break,
            }
        }

        KeyValues { sorted_set: merged }
    }

    /// Returns an iterator over the key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.sorted_set.iter()
    }

    /// Returns the number of key-value pairs
    pub fn len(&self) -> usize {
        self.sorted_set.len()
    }

    /// Checks if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.sorted_set.is_empty()
    }
}

impl<'a, T> IntoIterator for &'a KeyValues<T>
where
    T: KeyValue,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.sorted_set.iter()
    }
}

impl<T> IntoIterator for KeyValues<T>
where
    T: KeyValue,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.sorted_set.into_iter()
    }
}

impl<T> std::fmt::Display for KeyValues<T>
where
    T: KeyValue,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .sorted_set
            .iter()
            .map(|kv| format!("{}={}", kv.key(), kv.value()))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "[{}]", s)
    }
}
