pub struct Document {
    pub title: String,
    pub content: String,
}

impl Document {
    pub fn new(title: String, content: String) -> Self {
        Self { title, content }
    }
}
