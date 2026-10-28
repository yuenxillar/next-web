use crate::core::cache::read_cache::ReadCache;

#[derive(Clone)]
pub struct MapCache {
    cache: Vec<String>,
}

impl ReadCache for MapCache {
    fn init(&mut self) {}

    fn put(&mut self, value: String) {
        self.cache.push(value);
    }

    fn get(&self, key: usize) -> Option<&str> {
        self.cache.get(key).map(AsRef::as_ref)
    }

    fn put_finished(&mut self) {}

    fn destory(&mut self) {}
}
