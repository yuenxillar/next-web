use crate::core::cache::read_cache::ReadCache;

#[derive(Clone)]
pub struct XlsCache {}

impl ReadCache for XlsCache {
    fn init(&mut self) {}

    fn put(&mut self, value: String) {}

    fn get(&self, key: usize) -> Option<&str> {
        // self.cache.get(key).map(AsRef::as_ref)
        todo!()
    }

    fn put_finished(&mut self) {}

    fn destory(&mut self) {}
}
