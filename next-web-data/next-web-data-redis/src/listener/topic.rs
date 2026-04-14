use crate::listener::{channel_topic::ChannelTopic, pattern_topic::PatternTopic};

/// Topic for a Redis message. Acts a high-level abstraction on top of Redis low-level channels or patterns.
pub trait Topic {
    /// Create a new ChannelTopic for channel subscriptions.
    fn channel<S>(channel_name: S) -> ChannelTopic
    where
        S: Into<String>,
    {
        return ChannelTopic::new(channel_name);
    }

    /// Create a new PatternTopic for channel subscriptions based on a pattern.
    fn pattern<S>(pattern: S) -> PatternTopic
    where
        S: Into<String>,
    {
        PatternTopic::new(pattern)
    }

    // Returns the topic (as a String).
    fn get_topic(&self) -> &str;
}

impl Topic for String {
    fn get_topic(&self) -> &str {
        self.as_str()
    }
}

impl Topic for &str {
    fn get_topic(&self) -> &str {
        self
    }
}
