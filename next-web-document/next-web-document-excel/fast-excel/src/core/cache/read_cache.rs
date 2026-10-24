pub trait ReadCache {
    fn init(&mut self);

    fn put(&mut self, value: String);

    fn get(&self, key: usize) -> Option<&str>;

    fn put_finished(&mut self);

    fn destory(&mut self);
}
