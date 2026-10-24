use crate::listener::topic::Topic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternTopic {
    name: String,
}

impl PatternTopic {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Topic for PatternTopic {
    fn get_topic(&self) -> &str {
        self.name.as_ref()
    }
}
