pub struct TeraContext {
    pub inner: tera::Context,
}

impl TeraContext {
    pub fn init() -> Self {
        let inner = tera::Context::new();
        Self { inner }
    }

    pub fn set_value(&mut self, key: &str, value: &str) {
        self.inner.insert(key, value);
    }
}
