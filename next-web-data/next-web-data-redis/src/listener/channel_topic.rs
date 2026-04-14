use crate::listener::topic::Topic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelTopic {
    name: String,
}

impl ChannelTopic {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Topic for ChannelTopic {
    fn get_topic(&self) -> &str {
        self.name.as_ref()
    }
}
