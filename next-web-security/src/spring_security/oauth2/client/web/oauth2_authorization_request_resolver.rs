//! The resolver that turns an authorization request URI into the
//! authorization request of the named client registration.

use std::sync::Arc;

use next_web_core::{traits::http::http_request::HttpRequest, Uuid};

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{ClientRegistration, ClientRegistrationRepository, OAuth2AuthorizationRequest},
};

pub trait OAuth2AuthorizationRequestResolver: Send + Sync {
    fn resolve(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError>;

    fn resolve_with_registration_id(
        &self,
        request: &dyn HttpRequest,
        client_registration_id: &str,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError>;
}

#[derive(Clone)]
pub struct DefaultOAuth2AuthorizationRequestResolver {
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    authorization_request_base_uri: String,
}

impl DefaultOAuth2AuthorizationRequestResolver {
    pub fn new(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorization_request_base_uri: impl Into<String>,
    ) -> Self {
        let authorization_request_base_uri = authorization_request_base_uri.into();
        assert!(
            !authorization_request_base_uri.trim().is_empty(),
            "authorization_request_base_uri cannot be empty"
        );
        Self {
            client_registration_repository,
            authorization_request_base_uri,
        }
    }

    fn registration_id(&self, request: &dyn HttpRequest) -> Option<String> {
        extract_registration_id(request.path(), &self.authorization_request_base_uri)
    }

    fn authorization_request_for(
        &self,
        client_registration: ClientRegistration,
    ) -> OAuth2AuthorizationRequest {
        let state = Uuid::new_v4().to_string();
        // A client that uses a proof key sends the digest of a verifier it
        // keeps, and the verifier is what it exchanges the code with.
        let proof_key = client_registration
            .require_proof_key()
            .then(|| ProofKey::generate());
        // An OpenID Connect client sends a nonce, and the identifier token the
        // grant receives has to carry it back.
        let nonce = client_registration
            .scopes()
            .iter()
            .any(|scope| scope == "openid")
            .then(|| Uuid::new_v4().to_string());
        let authorization_request_uri = authorization_request_uri(
            &client_registration,
            &state,
            proof_key.as_ref(),
            nonce.as_deref(),
        );
        let mut authorization_request = OAuth2AuthorizationRequest::new(
            client_registration.registration_id().to_string(),
            authorization_request_uri,
            state,
            client_registration.redirect_uri().map(ToOwned::to_owned),
            client_registration.scopes().to_vec(),
        );
        if let Some(nonce) = nonce {
            authorization_request = authorization_request.set_nonce(nonce);
        }

        match proof_key {
            Some(proof_key) => authorization_request.set_code_verifier(proof_key.verifier),
            None => authorization_request,
        }
    }
}

/// The proof key a client uses for the authorization code grant.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ProofKey {
    verifier: String,
    challenge: String,
    method: &'static str,
}

impl ProofKey {
    /// Generates a proof key with its challenge.
    fn generate() -> Self {
        use sha2::{Digest, Sha256};

        // The verifier is a random value of the characters the specification
        // allows, and the challenge is its base64url encoded SHA-256 digest.
        let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let challenge = crate::oauth2::jose::base64_url(&Sha256::digest(verifier.as_bytes()));

        Self {
            verifier,
            challenge,
            method: "S256",
        }
    }
}

impl OAuth2AuthorizationRequestResolver for DefaultOAuth2AuthorizationRequestResolver {
    fn resolve(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError> {
        let Some(registration_id) = self.registration_id(request) else {
            // Not an authorization request URI: let the filter chain continue.
            return Ok(None);
        };
        self.resolve_with_registration_id(request, &registration_id)
    }

    fn resolve_with_registration_id(
        &self,
        _request: &dyn HttpRequest,
        client_registration_id: &str,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError> {
        let registration = self
            .client_registration_repository
            .find_by_registration_id(client_registration_id)
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    format!(
                        "Invalid Client Registration with Id: {}",
                        client_registration_id
                    ),
                    AuthenticationErrorKind::InvalidClientRegistrationId,
                )
            })?;
        // The Java resolver throws IllegalArgumentException for non-authorization-code
        // grants; returning Ok(None) is a documented simplification of this port.
        if !registration.is_authorization_code() {
            return Ok(None);
        }
        Ok(Some(self.authorization_request_for(registration)))
    }
}

pub fn extract_registration_id(path: &str, authorization_request_base_uri: &str) -> Option<String> {
    let prefix = format!("{}/", authorization_request_base_uri.trim_end_matches('/'));
    let start = path.rfind(&prefix)?;
    let registration_id = &path[start + prefix.len()..];
    let registration_id = registration_id.split('/').next().unwrap_or_default();
    if registration_id.is_empty() {
        None
    } else {
        Some(registration_id.to_string())
    }
}

fn authorization_request_uri(
    client_registration: &ClientRegistration,
    state: &str,
    proof_key: Option<&ProofKey>,
    nonce: Option<&str>,
) -> String {
    let mut params = Vec::new();
    params.push(("response_type", "code".to_string()));
    if !client_registration.client_id().is_empty() {
        params.push(("client_id", client_registration.client_id().to_string()));
    }
    if !client_registration.scopes().is_empty() {
        params.push(("scope", client_registration.scopes().join(" ")));
    }
    if let Some(redirect_uri) = client_registration.redirect_uri() {
        params.push(("redirect_uri", redirect_uri.to_string()));
    }
    if let Some(proof_key) = proof_key {
        params.push(("code_challenge", proof_key.challenge.clone()));
        params.push(("code_challenge_method", proof_key.method.to_string()));
    }
    if let Some(nonce) = nonce {
        params.push(("nonce", nonce.to_string()));
    }
    params.push(("state", state.to_string()));

    let separator = if client_registration.authorization_uri().contains('?') {
        '&'
    } else {
        '?'
    };
    let query = params
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, urlencoding::encode(&value)))
        .collect::<Vec<_>>()
        .join("&");

    format!(
        "{}{}{}",
        client_registration.authorization_uri(),
        separator,
        query
    )
}
