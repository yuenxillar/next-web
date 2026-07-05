#[derive(Clone)]
pub struct Cookie {
    // _inner: headers::Cookie,
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: Option<String>) -> Self {
        Self {}
    }

    pub fn set_max_age(&mut self, max_age: i32) {}

    pub fn set_secure(&mut self, secure: bool) {}

    pub fn set_path(&mut self, path: impl Into<String>) {}

    pub fn set_http_only(&mut self, http_only: bool) {}

    pub fn name(&self) -> &str {
        todo!()
    }

    pub fn value(&self) -> &str {
        todo!()
    }
}
