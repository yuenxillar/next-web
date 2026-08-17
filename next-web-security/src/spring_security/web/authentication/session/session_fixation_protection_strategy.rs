use std::{
    collections::HashMap,
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
};

use next_web_core::{
    anys::any_value::AnyValue,
    traits::http::{http_request::HttpRequest, HttpSession},
};
use tracing::debug;

use crate::web::authentication::session::{
    BaseSessionFixationProtectionStrategy, BaseSessionFixationProtectionStrategyExt,
};

/// Uses HttpServletRequest.invalidate() to protect against session fixation attacks.
///
/// Creates a new session for the newly authenticated user if they already have a session
/// (as a defence against session-fixation protection attacks), and copies their session
/// attributes across to the new session. The copying of the attributes can be disabled by
/// setting `migrateSessionAttributes` to `false` (note that even in this case, internal
/// NEXT Security attributes will still be migrated to the new session).
///
/// This approach will only be effective if your servlet container always assigns a new
/// session Id when a session is invalidated and a new session created by calling
/// `HttpRequest#get_session()`.
#[derive(Clone)]
pub struct SessionFixationProtectionStrategy {
    /// Indicates that the session attributes of an existing session should be migrated to the new session. Defaults to true.
    migrate_session_attributes: bool,

    inner: BaseSessionFixationProtectionStrategy,
}

impl SessionFixationProtectionStrategy {
    /// Called to extract the existing attributes from the session, prior to invalidating
    /// it. If migrate attributes is set to `false`, only NEXT Security attributes will
    /// be retained. All application attributes will be discarded.
    ///
    /// You can override this method to control exactly what is transferred to the new
    /// session.
    ///
    /// Returns the list of session attributes which should be transferred to the new
    /// session.
    pub fn extract_attributes(&self, session: &dyn HttpSession) -> HashMap<String, AnyValue> {
        self.create_migrated_attribute_map(session)
    }

    /// Transfers the extracted attributes to the newly created session.
    fn transfer_attributes(
        &self,
        attributes: &HashMap<String, AnyValue>,
        new_session: &mut dyn HttpSession,
    ) {
        for (key, value) in attributes {
            new_session.set_attribute(key, value.clone());
        }
    }

    fn create_migrated_attribute_map(
        &self,
        session: &dyn HttpSession,
    ) -> HashMap<String, AnyValue> {
        let mut attributes_to_migrate = HashMap::new();
        for key in session.attribute_names() {
            if !self.migrate_session_attributes && !key.starts_with("NEXT_SECURITY_") {
                // Only retain NEXT Security attributes
                continue;
            }
            if let Some(value) = session.attribute(key) {
                attributes_to_migrate.insert(key.to_string(), value.clone());
            }
        }
        attributes_to_migrate
    }

    /// Defines whether attributes should be migrated to a new session or not. Has no
    /// effect if `extract_attributes` is overridden.
    ///
    /// Attributes used by NEXT Security (to store cached requests, for example) will
    /// still be retained by default, even if this value is set to `false`.
    pub fn set_migrate_session_attributes(&mut self, migrate_session_attributes: bool) {
        self.migrate_session_attributes = migrate_session_attributes;
    }
}

impl BaseSessionFixationProtectionStrategyExt for SessionFixationProtectionStrategy {
    fn apply_session_fixation<'a>(
        &'a self,
        request: &'a mut dyn HttpRequest,
    ) -> Option<&'a mut dyn HttpSession> {
        let mut attributes_to_migrate = HashMap::new();
        let mut max_inactive_interval_to_migrate = 0;

        if let Some(session) = request.session_mut(true) {
            let original_session_id = session.id().to_string();
            debug!(
                "Invalidating session with Id '{}' {} migrating attributes.",
                original_session_id,
                if self.migrate_session_attributes {
                    "and"
                } else {
                    "without"
                }
            );
            attributes_to_migrate = self.extract_attributes(&*session);
            max_inactive_interval_to_migrate = session.max_inactive_interval();
            session.invalidate();
        }

        // we now have a new session
        let mut new_session = request.session_mut(true);
        if let Some(session) = new_session.as_mut() {
            debug!("Started new session: {}", session.id());
            self.transfer_attributes(&attributes_to_migrate, &mut **session);
            if self.migrate_session_attributes {
                session.set_max_inactive_interval(max_inactive_interval_to_migrate);
            }
        }
        new_session
    }
}

impl Deref for SessionFixationProtectionStrategy {
    type Target = BaseSessionFixationProtectionStrategy;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SessionFixationProtectionStrategy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Debug for SessionFixationProtectionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionFixationProtectionStrategy")
            .field(
                "migrate_session_attributes",
                &self.migrate_session_attributes,
            )
            .field("inner", &"none")
            .finish()
    }
}

impl Default for SessionFixationProtectionStrategy {
    fn default() -> Self {
        Self {
            migrate_session_attributes: true,
            inner: BaseSessionFixationProtectionStrategy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, sync::Mutex};

    use next_web_core::anys::any_value::AnyValue;

    use super::SessionFixationProtectionStrategy;
    use next_web_core::traits::http::HttpSession;

    struct MockSession {
        id: String,
        attributes: Vec<(String, AnyValue)>,
        written: Mutex<Vec<(String, AnyValue)>>,
        max_inactive_interval: u64,
    }

    impl MockSession {
        fn new(attributes: Vec<(String, AnyValue)>) -> Self {
            Self {
                id: String::from("original"),
                attributes,
                written: Mutex::new(Vec::new()),
                max_inactive_interval: 0,
            }
        }

        fn written(&self) -> Vec<(String, AnyValue)> {
            self.written.lock().unwrap().clone()
        }
    }

    impl fmt::Display for MockSession {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "MockSession")
        }
    }

    impl HttpSession for MockSession {
        fn creation_time(&self) -> u64 {
            0
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn last_accessed_time(&self) -> u64 {
            0
        }

        fn set_max_inactive_interval(&mut self, interval: u64) {
            self.max_inactive_interval = interval;
        }

        fn max_inactive_interval(&self) -> u64 {
            self.max_inactive_interval
        }

        fn attribute(&self, name: &str) -> Option<&AnyValue> {
            self.attributes
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value)
        }

        fn attribute_names(&self) -> Vec<&str> {
            self.attributes
                .iter()
                .map(|(key, _)| key.as_str())
                .collect()
        }

        fn set_attribute(&self, name: &str, value: AnyValue) {
            self.written.lock().unwrap().push((name.to_string(), value));
        }

        fn remove_attribute(&self, _name: &str) {}

        fn invalidate(&mut self) {}

        fn is_new(&self) -> bool {
            false
        }
    }

    fn attributes(values: &[(&str, &str)]) -> Vec<(String, AnyValue)> {
        values
            .iter()
            .map(|(key, value)| (key.to_string(), AnyValue::from(*value)))
            .collect()
    }

    #[test]
    fn default_migrates_all_attributes() {
        let strategy = SessionFixationProtectionStrategy::default();
        let session = MockSession::new(attributes(&[
            ("name", "alice"),
            ("NEXT_SECURITY_CSRF", "token"),
        ]));

        let extracted = strategy.extract_attributes(&session);

        let keys: Vec<&str> = extracted.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, vec!["name", "NEXT_SECURITY_CSRF"]);
        assert_eq!(extracted["name"].as_str(), Some("alice"));
    }

    #[test]
    fn without_migration_keeps_only_next_security_attributes() {
        let mut strategy = SessionFixationProtectionStrategy::default();
        strategy.set_migrate_session_attributes(false);
        let session = MockSession::new(attributes(&[
            ("name", "alice"),
            ("NEXT_SECURITY_CSRF", "token"),
        ]));

        let extracted = strategy.extract_attributes(&session);

        let keys: Vec<&str> = extracted.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, vec!["NEXT_SECURITY_CSRF"]);
    }

    #[test]
    fn transfer_attributes_copies_into_new_session() {
        let strategy = SessionFixationProtectionStrategy::default();
        let mut new_session = MockSession::new(Vec::new());

        strategy.transfer_attributes(
            &attributes(&[("name", "alice"), ("NEXT_SECURITY_CSRF", "token")])
                .into_iter()
                .collect::<_>(),
            &mut new_session,
        );

        let written = new_session.written();
        let keys: Vec<&str> = written.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, vec!["name", "NEXT_SECURITY_CSRF"]);
        assert_eq!(written[0].1.as_str(), Some("alice"));
    }
}
