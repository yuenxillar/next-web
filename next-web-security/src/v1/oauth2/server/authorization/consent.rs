//! The consents a resource owner granted to a client.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// The scopes a resource owner granted to a client.
///
/// The consent is what the authorization endpoint reads to decide whether it
/// has to ask the resource owner again: a request whose scopes are all part of
/// the consent of the client is granted without asking.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OAuth2AuthorizationConsent {
    registered_client_id: String,
    principal_name: String,
    authorities: Vec<String>,
}

impl OAuth2AuthorizationConsent {
    /// Creates a consent for a client and a resource owner.
    ///
    /// # Arguments
    ///
    /// * `registered_client_id` - The identifier of the client.
    /// * `principal_name` - The name of the resource owner.
    pub fn with_id(
        registered_client_id: impl Into<String>,
        principal_name: impl Into<String>,
    ) -> Self {
        Self {
            registered_client_id: registered_client_id.into(),
            principal_name: principal_name.into(),
            authorities: Vec::new(),
        }
    }

    /// Returns the identifier of the client.
    pub fn registered_client_id(&self) -> &str {
        &self.registered_client_id
    }

    /// Returns the name of the resource owner.
    pub fn principal_name(&self) -> &str {
        &self.principal_name
    }

    /// Returns the scopes the resource owner granted.
    pub fn authorities(&self) -> &[String] {
        &self.authorities
    }

    /// Returns whether the resource owner granted the given scope.
    ///
    /// # Arguments
    ///
    /// * `scope` - The scope to look up.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.authorities.iter().any(|authority| authority == scope)
    }

    /// Returns whether the resource owner granted every given scope.
    ///
    /// # Arguments
    ///
    /// * `scopes` - The scopes to look up.
    pub fn has_scopes(&self, scopes: &[String]) -> bool {
        scopes.iter().all(|scope| self.has_scope(scope))
    }

    /// Adds the scopes the resource owner granted, keeping the ones that were
    /// granted before.
    ///
    /// # Arguments
    ///
    /// * `scopes` - The scopes to add.
    pub fn grant_scopes<I, S>(&mut self, scopes: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for scope in scopes {
            let scope = scope.into();
            if !self.authorities.contains(&scope) {
                self.authorities.push(scope);
            }
        }
    }
}

/// Stores the consents the resource owners granted to the clients.
pub trait OAuth2AuthorizationConsentService: Send + Sync {
    /// Stores a consent, replacing the one it is built for.
    ///
    /// # Arguments
    ///
    /// * `authorization_consent` - The consent to store.
    fn save(&self, authorization_consent: OAuth2AuthorizationConsent);

    /// Removes a consent.
    ///
    /// # Arguments
    ///
    /// * `authorization_consent` - The consent to remove.
    fn remove(&self, authorization_consent: &OAuth2AuthorizationConsent);

    /// Returns the consent of a resource owner for a client, if one is stored.
    ///
    /// # Arguments
    ///
    /// * `registered_client_id` - The identifier of the client.
    /// * `principal_name` - The name of the resource owner.
    fn find_by_id(
        &self,
        registered_client_id: &str,
        principal_name: &str,
    ) -> Option<OAuth2AuthorizationConsent>;
}

/// An [`OAuth2AuthorizationConsentService`] that keeps its consents in memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryOAuth2AuthorizationConsentService {
    consents: Arc<RwLock<HashMap<(String, String), OAuth2AuthorizationConsent>>>,
}

impl OAuth2AuthorizationConsentService for InMemoryOAuth2AuthorizationConsentService {
    fn save(&self, authorization_consent: OAuth2AuthorizationConsent) {
        self.consents
            .write()
            .expect("the consents lock is not poisoned")
            .insert(
                (
                    authorization_consent.registered_client_id().to_string(),
                    authorization_consent.principal_name().to_string(),
                ),
                authorization_consent,
            );
    }

    fn remove(&self, authorization_consent: &OAuth2AuthorizationConsent) {
        self.consents
            .write()
            .expect("the consents lock is not poisoned")
            .remove(&(
                authorization_consent.registered_client_id().to_string(),
                authorization_consent.principal_name().to_string(),
            ));
    }

    fn find_by_id(
        &self,
        registered_client_id: &str,
        principal_name: &str,
    ) -> Option<OAuth2AuthorizationConsent> {
        self.consents
            .read()
            .expect("the consents lock is not poisoned")
            .get(&(registered_client_id.to_string(), principal_name.to_string()))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryOAuth2AuthorizationConsentService, OAuth2AuthorizationConsent,
        OAuth2AuthorizationConsentService,
    };

    #[test]
    fn a_consent_carries_the_scopes_of_a_client_and_a_resource_owner() {
        let mut consent = OAuth2AuthorizationConsent::with_id("demo-client", "user");
        consent.grant_scopes(["read", "write", "read"]);

        assert_eq!(consent.registered_client_id(), "demo-client");
        assert_eq!(consent.principal_name(), "user");
        assert_eq!(consent.authorities(), ["read", "write"]);
        assert!(consent.has_scope("read"));
        assert!(!consent.has_scope("admin"));
        assert!(consent.has_scopes(&["read".to_string(), "write".to_string()]));
        assert!(!consent.has_scopes(&["read".to_string(), "admin".to_string()]));
    }

    #[test]
    fn a_service_finds_the_consent_it_stores() {
        let service = InMemoryOAuth2AuthorizationConsentService::default();
        let mut consent = OAuth2AuthorizationConsent::with_id("demo-client", "user");
        consent.grant_scopes(["read"]);
        service.save(consent);

        assert!(service.find_by_id("demo-client", "user").is_some());
        assert!(service.find_by_id("demo-client", "other").is_none());

        let stored = service
            .find_by_id("demo-client", "user")
            .expect("the consent is stored");
        service.remove(&stored);
        assert!(service.find_by_id("demo-client", "user").is_none());
    }
}
