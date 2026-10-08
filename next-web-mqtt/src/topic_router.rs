use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::topic_listener::TopicListener;

/// Routes published topics to the listeners that subscribed to them.
///
/// An exact subscription (`sensors/room1`) is matched by name, and a wildcard
/// subscription (`sensors/+/temperature` or `alerts/#`) is matched against the
/// published topic. A topic is handed to every listener it matches.
#[derive(Clone, Default)]
pub struct TopicRouter {
    /// Listeners registered under an exact topic name.
    pub(crate) exact: HashMap<String, Vec<Arc<dyn TopicListener>>>,
    /// Listeners registered under a wildcard topic filter.
    pub(crate) match_: Vec<TopicRouteMatch>,
}

impl TopicRouter {
    /// Creates a router out of exact and wildcard subscriptions.
    pub fn new<T1, T2>(exact: T1, match_: T2) -> Self
    where
        T1: IntoIterator<Item = (String, Vec<Arc<dyn TopicListener>>)>,
        T2: IntoIterator<Item = TopicRouteMatch>,
    {
        let exact = exact.into_iter().collect();
        let match_ = match_.into_iter().collect();

        Self { exact, match_ }
    }

    /// Returns every listener whose subscription matches `topic`.
    ///
    /// The listeners registered under the exact topic name come first, followed
    /// by the listeners whose wildcard filter matches the topic. The returned
    /// vector is empty when no subscription matches.
    pub fn route(&self, topic: &str) -> Vec<&dyn TopicListener> {
        let exact = self
            .exact
            .get(topic)
            .into_iter()
            .flatten()
            .map(AsRef::as_ref);

        let wildcard = self
            .match_
            .iter()
            .filter(|route| route.matches(topic))
            .map(|route| route.listener.as_ref());

        exact.chain(wildcard).collect()
    }
}

/// A subscription filter and the listener it is registered for.
#[derive(Clone)]
pub struct TopicRouteMatch {
    /// Subscription filter, for example `sensor/+/temperature` or `test/#`.
    pub topic: Cow<'static, str>,
    /// Pre-classified match type for quick inspection and debugging.
    pub match_type: MatchType,
    /// Listener that receives the matching messages.
    pub listener: Arc<dyn TopicListener>,
}

/// MQTT topic filter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Multi-level wildcard filter containing `#`.
    Multilayer,
    /// Single-level wildcard filter containing `+` but no `#`.
    Singlelayer,
    /// `#`, which matches any non-system topic.
    Anything,
}

impl TopicRouteMatch {
    /// Builds a wildcard route from a filter and its listener.
    ///
    /// # Errors
    ///
    /// Returns the reason the filter is not a valid wildcard filter.
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

fn classify_filter(filter: &str) -> Result<MatchType, String> {
    validate_filter(filter)?;

    if filter == "#" {
        Ok(MatchType::Anything)
    } else if filter.contains('#') {
        Ok(MatchType::Multilayer)
    } else if filter.contains('+') {
        Ok(MatchType::Singlelayer)
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

    use super::{mqtt_filter_matches, MatchType, TopicRouter};
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

    /// A listener that is named after the filter it stands for, so that a test
    /// can tell the listeners of a route apart.
    #[derive(Clone)]
    struct NamedTopic(&'static str);

    #[async_trait]
    impl TopicListener for NamedTopic {
        fn topic(&self) -> &'static str {
            self.0
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
        assert_eq!(route.match_type, MatchType::Anything);
    }

    #[test]
    fn matches_multilevel_wildcard() {
        assert!(mqtt_filter_matches("sport/#", "sport"));
        assert!(mqtt_filter_matches("sport/#", "sport/"));
        assert!(mqtt_filter_matches("sport/#", "sport/tennis/player1"));
        assert!(!mqtt_filter_matches("sport/#", "finance/stock"));
        assert!(!mqtt_filter_matches("sport/#", "sportx"));
    }

    #[test]
    fn matches_singlelevel_wildcard() {
        assert!(mqtt_filter_matches(
            "sport/+/player1",
            "sport/tennis/player1"
        ));
        assert!(mqtt_filter_matches("sport/+", "sport/"));
        assert!(!mqtt_filter_matches("sport/+", "sport"));
        assert!(!mqtt_filter_matches("sport/+", "sport/tennis/player1"));
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

    /// Pushes a named listener under an exact topic.
    fn add_exact(router: &mut TopicRouter, topic: &str, listener: &'static str) {
        router
            .exact
            .entry(topic.to_owned())
            .or_default()
            .push(Arc::new(NamedTopic(listener)));
    }

    #[test]
    fn routes_to_every_listener_of_an_exact_topic() {
        let mut router = TopicRouter::default();
        add_exact(&mut router, "test/123", "first");
        add_exact(&mut router, "test/123", "second");

        assert_eq!(router.route("test/123").len(), 2);
    }

    #[test]
    fn routes_exact_and_wildcard_listeners_together() {
        let mut router = TopicRouter::default();
        add_exact(&mut router, "test/123", "test/123");
        router.match_.push(
            TopicRouteMatch::new("test/+", Arc::new(NamedTopic("test/+")))
                .expect("the filter is a valid single-level wildcard"),
        );

        let matched: Vec<&str> = router
            .route("test/123")
            .iter()
            .map(|listener| listener.topic())
            .collect();
        assert_eq!(matched, vec!["test/123", "test/+"]);

        // The wildcard matches a topic the exact listener does not.
        assert_eq!(router.route("test/other").len(), 1);
    }

    #[test]
    fn routes_nothing_when_no_subscription_matches() {
        let mut router = TopicRouter::default();
        add_exact(&mut router, "test/123", "test/123");

        assert!(router.route("other/1").is_empty());
    }
}
