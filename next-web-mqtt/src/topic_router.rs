use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::topic_listener::TopicListener;

/// MQTT topic router entry.
///
/// It stores one subscription filter plus the consumer that should receive a
/// published message when the filter matches the incoming topic name.
#[derive(Clone, Default)]
pub struct TopicRouter {
    pub(crate) exact: HashMap<String, Arc<dyn TopicListener>>,
    pub(crate) match_: Vec<TopicRouteMatch>,
}

impl TopicRouter {
    pub fn new<T1, T2>(exact: T1, match_: T2) -> Self
    where
        T1: IntoIterator<Item = (String, Arc<dyn TopicListener>)>,
        T2: IntoIterator<Item = TopicRouteMatch>,
    {
        let exact = exact.into_iter().collect();
        let match_ = match_.into_iter().collect();

        Self { exact, match_ }
    }

    pub fn route(&self, topic: &str) -> Option<&dyn TopicListener> {
        self.exact.get(topic).map(AsRef::as_ref).or_else(|| {
            self.match_
                .iter()
                .find(|route| route.matches(topic))
                .map(|route| route.listener.as_ref())
        })
    }
}

#[derive(Clone)]
pub struct TopicRouteMatch {
    /// Subscription filter, for example `sensor/+/temperature` or `test/#`.
    pub topic: Cow<'static, str>,
    /// Pre-classified match type for quick inspection and debugging.
    pub match_type: MacthType,
    /// Topic Listener.
    pub listener: Arc<dyn TopicListener>,
}

/// MQTT topic filter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacthType {
    /// Multi-level wildcard filter containing `#`.
    Multilayer,
    /// Single-level wildcard filter containing `+` but no `#`.
    Singlelayer,
    /// `#`, which matches any non-system topic.
    Anything,
}

impl TopicRouteMatch {
    pub fn new<M: Into<Cow<'static, str>>>(
        topic: M,
        listener: Arc<dyn TopicListener>,
    ) -> Result<Self, String> {
        let topic = topic.into();
        let match_type = classify_filter(&topic)?;

        Ok(Self {
            topic,
            match_type,
            listener,
        })
    }

    /// Returns `true` when the MQTT topic filter matches the published topic.
    pub fn matches(&self, incoming_topic: &str) -> bool {
        mqtt_filter_matches(&self.topic, incoming_topic)
    }
}

fn classify_filter(filter: &str) -> Result<MacthType, String> {
    validate_filter(filter)?;

    if filter == "#" {
        Ok(MacthType::Anything)
    } else if filter.contains('#') {
        Ok(MacthType::Multilayer)
    } else if filter.contains('+') {
        Ok(MacthType::Singlelayer)
    } else {
        Err(format!(
            "topic '{filter}' does not contain a supported wildcard pattern"
        ))
    }
}

fn validate_filter(filter: &str) -> Result<(), String> {
    if filter.is_empty() {
        return Err("topic filter cannot be empty".to_string());
    }

    let levels: Vec<&str> = filter.split('/').collect();
    let mut has_wildcard = false;

    for (index, level) in levels.iter().enumerate() {
        if level.contains('#') {
            has_wildcard = true;
            if *level != "#" {
                return Err(format!(
                    "invalid multi-level MQTT topic pattern '{filter}': '#' must occupy an entire level"
                ));
            }
            if index + 1 != levels.len() {
                return Err(format!(
                    "invalid multi-level MQTT topic pattern '{filter}': '#' must be the last level"
                ));
            }
        }

        if level.contains('+') {
            has_wildcard = true;
            if *level != "+" {
                return Err(format!(
                    "invalid single-level MQTT topic pattern '{filter}': '+' must occupy an entire level"
                ));
            }
        }
    }

    if !has_wildcard {
        return Err(format!(
            "topic '{filter}' does not contain a supported wildcard pattern"
        ));
    }

    Ok(())
}

fn mqtt_filter_matches(filter: &str, topic: &str) -> bool {
    if topic.is_empty() {
        return false;
    }

    // MQTT reserves `$`-prefixed topics for the server. Wildcard subscriptions
    // that start with `#` or `+` must not match them.
    if topic.starts_with('$') && (filter.starts_with('#') || filter.starts_with('+')) {
        return false;
    }

    let filter_levels: Vec<&str> = filter.split('/').collect();
    let topic_levels: Vec<&str> = topic.split('/').collect();
    let mut topic_index = 0usize;

    for filter_level in &filter_levels {
        match *filter_level {
            "#" => return true,
            "+" => {
                if topic_index >= topic_levels.len() {
                    return false;
                }
                topic_index += 1;
            }
            literal => {
                if topic_levels.get(topic_index).copied() != Some(literal) {
                    return false;
                }
                topic_index += 1;
            }
        }
    }

    topic_index == topic_levels.len()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{topic_listener::TopicListener, topic_router::TopicRouteMatch};

    use super::{mqtt_filter_matches, MacthType};
    use next_web_core::async_trait;

    #[derive(Clone)]
    struct DummyTopic;

    #[async_trait]
    impl TopicListener for DummyTopic {
        fn topic(&self) -> &'static str {
            "#"
        }

        async fn on_message(&self, _topic: &str, _message: &[u8]) {}
    }

    #[test]
    fn rejects_invalid_wildcard_positions() {
        assert!(TopicRouteMatch::new("sensor/#/temp", Arc::new(DummyTopic)).is_err());
        assert!(TopicRouteMatch::new("sensor+temp", Arc::new(DummyTopic)).is_err());
        assert!(TopicRouteMatch::new("sensor/room", Arc::new(DummyTopic)).is_err());
    }

    #[test]
    fn classifies_anything_route() {
        let route = TopicRouteMatch::new("#", Arc::new(DummyTopic)).unwrap();
        assert_eq!(route.match_type, MacthType::Anything);
    }

    #[test]
    fn matches_multilevel_wildcard() {
        assert!(mqtt_filter_matches("sport/#", "sport"));
        assert!(mqtt_filter_matches("sport/#", "sport/tennis/player1"));
        assert!(!mqtt_filter_matches("sport/#", "finance/stock"));
    }

    #[test]
    fn matches_singlelevel_wildcard() {
        assert!(mqtt_filter_matches(
            "sport/+/player1",
            "sport/tennis/player1"
        ));
        assert!(mqtt_filter_matches("sport/+", "sport/"));
        assert!(!mqtt_filter_matches("sport/+", "sport"));
        assert!(!mqtt_filter_matches(
            "sport/+/player1",
            "sport/tennis/player1/ranking"
        ));
    }

    #[test]
    fn system_topics_do_not_match_root_wildcards() {
        assert!(!mqtt_filter_matches("#", "$SYS/broker/uptime"));
        assert!(!mqtt_filter_matches(
            "+/broker/uptime",
            "$SYS/broker/uptime"
        ));
        assert!(mqtt_filter_matches("$SYS/#", "$SYS/broker/uptime"));
    }
}
