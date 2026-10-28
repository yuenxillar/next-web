//! The authorization requests a client pushed to the authorization server.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// An authorization request a client pushed to the authorization server.
///
/// The parameters are the parameters the client would have sent to the
/// authorization endpoint, and the request URI is the value the client sends
/// instead of them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PushedAuthorizationRequest {
    request_uri: String,
    registered_client_id: String,
    parameters: Vec<(String, String)>,
    expires_at: u64,
}

impl PushedAuthorizationRequest {
    /// Creates a pushed authorization request.
    ///
    /// # Arguments
    ///
    /// * `request_uri` - The identifier of the request.
    /// * `registered_client_id` - The registration the request belongs to.
    /// * `parameters` - The parameters of the authorization request.
    /// * `expires_at` - The time the request expires, in seconds since the epoch.
    pub fn new(
        request_uri: impl Into<String>,
        registered_client_id: impl Into<String>,
        parameters: Vec<(String, String)>,
        expires_at: u64,
    ) -> Self {
        Self {
            request_uri: request_uri.into(),
            registered_client_id: registered_client_id.into(),
            parameters,
            expires_at,
        }
    }

    /// Returns the identifier of the request.
    pub fn request_uri(&self) -> &str {
        &self.request_uri
    }

    /// Returns the registration the request belongs to.
    pub fn registered_client_id(&self) -> &str {
        &self.registered_client_id
    }

    /// Returns the parameters of the authorization request.
    pub fn parameters(&self) -> &[(String, String)] {
        &self.parameters
    }

    /// Returns the value of a parameter, if the request carries one.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the parameter.
    pub fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Returns the time the request expires, in seconds since the epoch.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Returns whether the request is expired.
    ///
    /// # Arguments
    ///
    /// * `now` - The current time, in seconds since the epoch.
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

/// Stores the authorization requests the clients pushed.
///
/// A pushed authorization request is consumed by the authorization endpoint,
/// so the request cannot be used twice.
pub trait OAuth2PushedAuthorizationRequestRepository: Send + Sync {
    /// Stores a pushed authorization request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to store.
    fn save(&self, request: PushedAuthorizationRequest);

    /// Returns the request with the given URI and removes it.
    ///
    /// # Arguments
    ///
    /// * `request_uri` - The identifier of the request.
    fn consume(&self, request_uri: &str) -> Option<PushedAuthorizationRequest>;

    /// Returns the request with the given URI without removing it.
    ///
    /// # Arguments
    ///
    /// * `request_uri` - The identifier of the request.
    fn find_by_request_uri(&self, request_uri: &str) -> Option<PushedAuthorizationRequest> {
        self.consume(request_uri)
            .inspect(|request| self.save(request.clone()))
    }
}

/// A [`OAuth2PushedAuthorizationRequestRepository`] that keeps its requests in
/// memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryOAuth2PushedAuthorizationRequestRepository {
    requests: Arc<RwLock<HashMap<String, PushedAuthorizationRequest>>>,
}

impl InMemoryOAuth2PushedAuthorizationRequestRepository {
    /// Creates a repository with the given requests.
    ///
    /// # Arguments
    ///
    /// * `requests` - The requests of the repository.
    pub fn new<I>(requests: I) -> Self
    where
        I: IntoIterator<Item = PushedAuthorizationRequest>,
    {
        let requests = requests
            .into_iter()
            .map(|request| (request.request_uri().to_string(), request))
            .collect();

        Self {
            requests: Arc::new(RwLock::new(requests)),
        }
    }
}

impl OAuth2PushedAuthorizationRequestRepository
    for InMemoryOAuth2PushedAuthorizationRequestRepository
{
    fn save(&self, request: PushedAuthorizationRequest) {
        self.requests
            .write()
            .expect("the pushed authorization requests lock is not poisoned")
            .insert(request.request_uri().to_string(), request);
    }

    fn consume(&self, request_uri: &str) -> Option<PushedAuthorizationRequest> {
        self.requests
            .write()
            .expect("the pushed authorization requests lock is not poisoned")
            .remove(request_uri)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryOAuth2PushedAuthorizationRequestRepository,
        OAuth2PushedAuthorizationRequestRepository, PushedAuthorizationRequest,
    };

    #[test]
    fn a_request_is_consumed_only_once() {
        let repository = InMemoryOAuth2PushedAuthorizationRequestRepository::default();
        repository.save(PushedAuthorizationRequest::new(
            "urn:ietf:params:oauth:request_uri:1",
            "demo-client",
            vec![("scope".to_string(), "read".to_string())],
            4_102_444_800,
        ));

        let request = repository
            .consume("urn:ietf:params:oauth:request_uri:1")
            .expect("the request is stored");

        assert_eq!(request.parameter("scope"), Some("read"));
        assert_eq!(request.registered_client_id(), "demo-client");
        assert!(repository
            .consume("urn:ietf:params:oauth:request_uri:1")
            .is_none());
    }

    #[test]
    fn an_expired_request_is_reported() {
        let request = PushedAuthorizationRequest::new(
            "urn:ietf:params:oauth:request_uri:1",
            "demo-client",
            Vec::new(),
            100,
        );

        assert!(request.is_expired(100));
        assert!(!request.is_expired(99));
    }
}
