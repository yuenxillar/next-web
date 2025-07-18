use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
};
// 哈希槽总数，默认为 16384。
///
/// This is the number of slots used in the hash slot system.
pub const SLOT_COUNT: usize = 16384;

#[derive(Clone)]
pub struct HashSlot<'a> {
    slots: Vec<Cow<'a, str>>,
    slot_count: usize,
}

impl<'a> HashSlot<'a> {
    /// 创建一个新的 HashSlot 实例。
    ///
    /// # 参数
    /// - `nodes`: 节点列表。
    /// - `slot_count`: 哈希槽的数量。
    ///
    /// # 返回值
    /// 返回一个初始化好的 HashSlot 实例。
    ///
    /// Creates a new instance of HashSlot.
    ///
    /// # Arguments
    /// - `nodes`: A list of nodes.
    /// - `slot_count`: The number of hash slots.
    ///
    /// # Returns
    /// An initialized instance of HashSlot.
    pub fn new<T: Into<Cow<'a, str>>>(nodes: Vec<T>, slot_count: usize) -> Self {
        let mut slots = vec![Cow::Borrowed(""); slot_count];

        let nodes = nodes.into_iter().map(|n| n.into()).collect::<Vec<_>>();
        let node_count = nodes.len();
        for slot in 0..slot_count {
            let node_idx = slot % node_count;
            slots[slot] = nodes[node_idx].clone();
        }
        HashSlot { slots, slot_count }
    }

    /// 根据给定的节点列表创建一个新的 HashSlot 实例，默认使用 SLOT_COUNT 作为哈希槽数量。
    ///
    /// # 参数
    /// - `nodes`: 节点列表。
    ///
    /// # 返回值
    /// 返回一个初始化好的 HashSlot 实例。
    ///
    /// Creates a new instance of HashSlot with default slot count.
    ///
    /// # Arguments
    /// - `nodes`: A list of nodes.
    ///
    /// # Returns
    /// An initialized instance of HashSlot with default slot count.
    pub fn from_nodes<T: Into<Cow<'a, str>>>(nodes: Vec<T>) -> Self {
        HashSlot::new(nodes.into_iter().map(|n| n.into()).collect(), SLOT_COUNT)
    }

    /// 根据提供的键获取对应的节点。
    ///
    /// # 参数
    /// - `key`: 键值。
    ///
    /// # 返回值
    /// 返回与键关联的节点名称。
    ///
    /// Gets the node associated with the given key.
    ///
    /// # Arguments
    /// - `key`: The key value.
    ///
    /// # Returns
    /// The name of the node associated with the key.
    pub fn get_node(&self, key: &str) -> &str {
        let slot = self.get_slot(key);
        &self.slots[slot]
    }

    /// 计算给定键的哈希槽位置。
    ///
    /// # 参数
    /// - `key`: 键值。
    ///
    /// # 返回值
    /// 返回计算出的哈希槽位置。
    ///
    /// Calculates the hash slot position for the given key.
    ///
    /// # Arguments
    /// - `key`: The key value.
    ///
    /// # Returns
    /// The calculated hash slot position.
    pub fn get_slot(&self, key: impl AsRef<str>) -> usize {
        (self.hash(key.as_ref()) as usize) % self.slot_count
    }

    /// 重新分片操作，更新哈希槽到节点的映射关系。
    ///
    /// # 参数
    /// - `nodes`: 新的节点列表。
    ///
    /// Resharding operation to update the mapping from hash slots to nodes.
    ///
    /// # Arguments
    /// - `nodes`: A new list of nodes.
    pub fn reshard<T: Into<Cow<'a, str>>>(&mut self, nodes: Vec<T>) {
        let new_nodes: Vec<_> = nodes.into_iter().map(|n| n.into()).collect();

        let node_count = new_nodes.len();
        for slot in 0..self.slot_count {
            let node_idx = slot % node_count;
            self.slots[slot] = new_nodes[node_idx].clone();
        }
    }

    /// 获取当前哈希槽的数量。
    ///
    /// # 返回值
    /// 返回当前哈希槽的数量。
    ///
    /// Gets the current number of hash slots.
    ///
    /// # Returns
    /// The current number of hash slots.
    pub fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// 使用默认哈希器对键进行哈希计算。
    ///
    /// # 参数
    /// - `key`: 键值。
    ///
    /// # 返回值
    /// 返回计算出的哈希值。
    ///
    /// Computes a hash value for the given key using the default hasher.
    ///
    /// # Arguments
    /// - `key`: The key value.
    ///
    /// # Returns
    /// The computed hash value.
    fn hash(&self, key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod hash_slot_test {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn test_hash_slot() {
        let nodes = vec![
            Cow::Borrowed("node1"),
            Cow::Borrowed("node2"),
            Cow::Borrowed("node3"),
        ];
        let mut hash_slot = HashSlot::from_nodes(nodes);
        println!("node is: {:?}", hash_slot.get_node("userid=1"));
        println!("node is: {:?}", hash_slot.get_node("userid=1.5"));
        println!("node is: {:?}", hash_slot.get_node("userid=2"));

        hash_slot.reshard(vec![
            "192.168.1.1:6379",
            "192.168.1.2:6379",
            "192.168.1.3:6379",
        ]);

        let keys = ["user:1001", "order:2002", "product:3003", "cart:4004"];
        for key in &keys {
            println!("{} -> {}", key, hash_slot.get_node(key));
        }
    }
}
