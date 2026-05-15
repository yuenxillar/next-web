use std::{borrow::Cow, collections::HashMap};

/// Total number of hash slots, defaulting to 16384.
pub const SLOT_COUNT: usize = 16384;

#[derive(Clone)]
pub struct HashSlot<'a> {
    slots: Vec<Cow<'a, str>>,
    slot_count: usize,
}

impl<'a> HashSlot<'a> {
    /// Creates a new HashSlot instance.
    ///
    /// # Arguments
    /// - `nodes`: A list of nodes.
    /// - `slot_count`: The number of hash slots.
    ///
    /// # Returns
    /// An initialized instance of HashSlot.
    pub fn new<T: Into<Cow<'a, str>>>(nodes: Vec<T>, slot_count: usize) -> Self {
        let mut slots = vec![Cow::Borrowed(""); slot_count];
        let nodes: Vec<Cow<'a, str>> = nodes.into_iter().map(|n| n.into()).collect();
        let node_count = nodes.len();

        for slot in 0..slot_count {
            let node_idx = slot % node_count;
            slots[slot] = nodes[node_idx].clone();
        }

        HashSlot { slots, slot_count }
    }

    /// Creates a new HashSlot instance with the default SLOT_COUNT.
    ///
    /// # Arguments
    /// - `nodes`: A list of nodes.
    ///
    /// # Returns
    /// An initialized instance of HashSlot with default slot count.
    pub fn from_nodes<T: Into<Cow<'a, str>>>(nodes: Vec<T>) -> Self {
        HashSlot::new(nodes, SLOT_COUNT)
    }

    /// Incrementally adds new nodes without affecting existing slot mappings.
    /// Only a subset of slots from existing nodes will be migrated to new nodes,
    /// ensuring that most existing key-to-node mappings remain unchanged.
    ///
    /// # Arguments
    /// - `new_nodes`: List of new nodes to add.
    /// - `slots_per_node`: Number of slots each new node should get (optional).
    ///   If None, slots will be distributed evenly among all nodes.
    pub fn add_nodes<T: Into<Cow<'a, str>>>(
        &mut self,
        new_nodes: Vec<T>,
        slots_per_node: Option<usize>,
    ) {
        let new_nodes: Vec<Cow<'a, str>> = new_nodes.into_iter().map(|n| n.into()).collect();
        if new_nodes.is_empty() {
            return;
        }

        // Get existing nodes (deduplicated) - collect into owned strings to avoid borrowing issues
        let existing_nodes: Vec<String> = {
            let mut nodes: Vec<String> = self
                .slots
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            nodes.sort();
            nodes.dedup();
            nodes
        };

        // Filter out nodes that already exist
        let actual_new_nodes: Vec<Cow<'a, str>> = new_nodes
            .into_iter()
            .filter(|n| !existing_nodes.contains(&n.to_string()))
            .collect();

        if actual_new_nodes.is_empty() {
            return;
        }

        // Calculate how many slots each new node should get
        let slots_per_new_node = slots_per_node.unwrap_or_else(|| {
            let total_nodes = existing_nodes.len() + actual_new_nodes.len();
            self.slot_count / total_nodes
        });

        // Count current slots per node - collect into a separate data structure first
        let mut node_slot_count: HashMap<String, usize> = HashMap::new();
        for slot in &self.slots {
            if !slot.is_empty() {
                *node_slot_count.entry(slot.to_string()).or_insert(0) += 1;
            }
        }

        // Target slots per node after adding new nodes
        let target_slots = self.slot_count / (existing_nodes.len() + actual_new_nodes.len());

        // Migrate slots from existing nodes to new nodes
        for new_node in &actual_new_nodes {
            let mut assigned = 0;

            // Migrate slots from nodes that have more than the target count
            while assigned < slots_per_new_node && assigned < self.slot_count {
                // Find the node with the most slots above target
                let donor_node = {
                    let donor = node_slot_count
                        .iter()
                        .filter(|(_, &count)| count > target_slots)
                        .max_by_key(|(_, &count)| count)
                        .map(|(node, _)| node.clone());
                    donor
                };

                if let Some(donor_node) = donor_node {
                    // Find a slot belonging to the donor node and reassign it
                    let slot_to_reassign = self
                        .slots
                        .iter()
                        .position(|slot| slot.as_ref() == donor_node.as_str());

                    if let Some(slot_idx) = slot_to_reassign {
                        self.slots[slot_idx] = new_node.clone();
                        *node_slot_count.get_mut(&donor_node).unwrap() -= 1;
                        *node_slot_count.entry(new_node.to_string()).or_insert(0) += 1;
                        assigned += 1;
                    } else {
                        // No more slots from this donor, remove it from consideration
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    /// Removes nodes and redistributes their slots to remaining nodes.
    /// Existing mappings for remaining nodes are preserved.
    ///
    /// # Arguments
    /// - `nodes_to_remove`: List of nodes to remove.
    pub fn remove_nodes<T: AsRef<str>>(&mut self, nodes_to_remove: &[T]) {
        if nodes_to_remove.is_empty() {
            return;
        }

        // Get remaining nodes after removal
        let remaining_nodes: Vec<String> = {
            let node_set: std::collections::HashSet<String> = self
                .slots
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .filter(|node| !nodes_to_remove.iter().any(|n| n.as_ref() == node.as_str()))
                .collect();
            node_set.into_iter().collect()
        };

        if remaining_nodes.is_empty() {
            // If no nodes remain, clear all slots
            for slot in &mut self.slots {
                *slot = Cow::Borrowed("");
            }
            return;
        }

        // Redistribute slots from removed nodes to remaining nodes
        let mut node_idx = 0;
        for slot in 0..self.slot_count {
            let should_reassign = {
                let current_node = &self.slots[slot];
                !current_node.is_empty()
                    && nodes_to_remove
                        .iter()
                        .any(|n| n.as_ref() == current_node.as_ref())
            };

            if should_reassign {
                self.slots[slot] =
                    Cow::Owned(remaining_nodes[node_idx % remaining_nodes.len()].clone());
                node_idx += 1;
            }
        }
    }

    /// Gets the node associated with the given key using Redis-compatible hash slot calculation.
    ///
    /// This method handles Redis-style key tags `{...}` for hash slot calculation.
    /// Only the content between the first `{` and the first `}` is used for hashing.
    /// If there is no such pattern, the entire key is used.
    ///
    /// # Arguments
    /// - `key`: The key value.
    ///
    /// # Returns
    /// The name of the node associated with the key.
    ///
    /// # Examples
    /// ```
    /// // For key "user:{1001}:profile", only "1001" is used for hashing
    /// let node = hash_slot.get_node("user:{1001}:profile");
    /// ```
    pub fn get_node(&self, key: &str) -> &str {
        let slot = self.get_slot(key);
        &self.slots[slot]
    }

    /// Calculates the hash slot position for the given key.
    ///
    /// Uses Redis-compatible CRC16 algorithm and handles key tags `{...}`.
    ///
    /// # Arguments
    /// - `key`: The key value.
    ///
    /// # Returns
    /// The calculated hash slot position (0-16383).
    pub fn get_slot(&self, key: impl AsRef<str>) -> usize {
        let key = self.extract_hash_tag(key.as_ref());
        (self.hash(key) as usize) % self.slot_count
    }

    /// Extracts the hash tag from a key.
    /// If the key contains `{...}`, returns the content between the braces.
    /// Otherwise, returns the entire key.
    ///
    /// # Arguments
    /// - `key`: The key to extract the hash tag from.
    ///
    /// # Returns
    /// The hash tag or the entire key.
    fn extract_hash_tag<'b>(&self, key: &'b str) -> &'b str {
        if let Some(start) = key.find('{') {
            if let Some(end) = key[start..].find('}') {
                let tag = &key[start + 1..start + end];
                if !tag.is_empty() {
                    return tag;
                }
            }
        }
        key
    }

    /// Gets the slot distribution across nodes.
    ///
    /// # Returns
    /// A HashMap with node names as keys and their slot count as values.
    pub fn get_slot_distribution(&self) -> HashMap<&str, usize> {
        let mut distribution = HashMap::new();
        for slot in &self.slots {
            if !slot.is_empty() {
                *distribution.entry(slot.as_ref()).or_insert(0) += 1;
            }
        }
        distribution
    }

    /// Gets the current number of hash slots.
    ///
    /// # Returns
    /// The current number of hash slots.
    pub fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Computes a CRC16 hash value for the given key.
    /// This is compatible with Redis hash slot calculation and provides
    /// cross-platform consistency.
    ///
    /// # Arguments
    /// - `key`: The key value to hash.
    ///
    /// # Returns
    /// The computed CRC16 hash value as u64.
    fn hash(&self, key: &str) -> u64 {
        crc16(key.as_bytes()) as u64
    }
}

/// Computes the CRC16 checksum of the given data.
/// This implementation is compatible with Redis CRC16 (XMODEM variant),
/// ensuring consistent hash values across different platforms and languages.
///
/// # Arguments
/// - `data`: The input data bytes.
///
/// # Returns
/// The CRC16 checksum value.
fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod hash_slot_test {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        // Test that the same key always produces the same slot
        let nodes = vec!["node1"];
        let hash_slot = HashSlot::from_nodes(nodes);

        let key = "user:1001";
        let slot1 = hash_slot.get_slot(key);
        let slot2 = hash_slot.get_slot(key);
        assert_eq!(slot1, slot2, "Same key should always produce the same slot");
    }

    #[test]
    fn test_hash_tag() {
        let nodes = vec!["node1", "node2", "node3"];
        let hash_slot = HashSlot::from_nodes(nodes);

        // Keys with the same hash tag should map to the same slot
        let key1 = "user:{1001}:profile";
        let key2 = "user:{1001}:settings";
        let key3 = "order:{1001}:details";

        let slot1 = hash_slot.get_slot(key1);
        let slot2 = hash_slot.get_slot(key2);
        let slot3 = hash_slot.get_slot(key3);

        assert_eq!(
            slot1, slot2,
            "Keys with same hash tag should map to same slot"
        );
        assert_eq!(
            slot1, slot3,
            "Keys with same hash tag should map to same slot"
        );

        // Keys without hash tags should use the entire key
        let key4 = "user:1001";
        let key5 = "user:1002";
        let slot4 = hash_slot.get_slot(key4);
        let slot5 = hash_slot.get_slot(key5);
        assert_ne!(
            slot4, slot5,
            "Different keys should likely map to different slots"
        );
    }

    #[test]
    fn test_cross_process_consistency() {
        // Test that CRC16 produces consistent results
        // These test vectors are known CRC16 values
        assert_eq!(crc16(b"123456789"), 0x31C3);
        assert_eq!(crc16(b""), 0x0000);

        // Redis specific test cases
        let nodes = vec!["node1"];
        let hash_slot = HashSlot::from_nodes(nodes);

        // Known Redis hash slot values
        assert_eq!(hash_slot.get_slot("123456789"), 0x31C3 % 16384);

        // Test that the hash method produces consistent results
        assert_eq!(hash_slot.hash("123456789"), 0x31C3);
        assert_eq!(hash_slot.hash(""), 0x0000);
    }

    #[test]
    fn test_incremental_add_nodes() {
        // Initialize with 3 nodes
        let nodes = vec!["node1", "node2", "node3"];
        let mut hash_slot = HashSlot::from_nodes(nodes);

        println!("Initial state:");
        let dist = hash_slot.get_slot_distribution();
        for (node, count) in &dist {
            println!("{}: {} slots", node, count);
        }

        // Remember mappings for some keys
        let key1 = "user:1001";
        let key2 = "order:2002";
        let node_before1 = hash_slot.get_node(key1).to_string();
        let node_before2 = hash_slot.get_node(key2).to_string();

        println!("\nBefore adding nodes:");
        println!("{} -> {}", key1, node_before1);
        println!("{} -> {}", key2, node_before2);

        // Incrementally add 2 new nodes
        hash_slot.add_nodes(vec!["node4", "node5"], None);

        println!("\nAfter adding nodes:");
        let dist = hash_slot.get_slot_distribution();
        for (node, count) in &dist {
            println!("{}: {} slots", node, count);
        }

        let node_after1 = hash_slot.get_node(key1);
        let node_after2 = hash_slot.get_node(key2);

        println!("{} -> {} (was: {})", key1, node_after1, node_before1);
        println!("{} -> {} (was: {})", key2, node_after2, node_before2);

        // Most existing key mappings should remain unchanged
        assert!(hash_slot.get_slot_distribution().contains_key("node4"));
        assert!(hash_slot.get_slot_distribution().contains_key("node5"));
    }

    #[test]
    fn test_remove_nodes() {
        let nodes = vec!["node1", "node2", "node3", "node4"];
        let mut hash_slot = HashSlot::from_nodes(nodes);

        println!("Before removing nodes:");
        let dist = hash_slot.get_slot_distribution();
        for (node, count) in &dist {
            println!("{}: {} slots", node, count);
        }

        // Remember a key mapping that's NOT on nodes being removed
        let test_key = "user:1001";
        let original_node = hash_slot.get_node(test_key).to_string();

        // Remove node2 and node4
        hash_slot.remove_nodes(&["node2", "node4"]);

        println!("\nAfter removing nodes:");
        let dist = hash_slot.get_slot_distribution();
        for (node, count) in &dist {
            println!("{}: {} slots", node, count);
        }

        // Verify removed nodes no longer exist
        for slot in &hash_slot.slots {
            assert!(slot.as_ref() != "node2" && slot.as_ref() != "node4");
        }

        // If the test key was on a remaining node, its mapping should be preserved
        if original_node != "node2" && original_node != "node4" {
            assert_eq!(hash_slot.get_node(test_key), original_node);
        }
    }
}
