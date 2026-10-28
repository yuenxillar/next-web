//! The authorizations the authorization server issued and grants.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use super::{
    client::AuthorizationGrantType,
    token::{now, OAuth2AccessToken, OAuth2RefreshToken, OAuth2TokenType},
};

/// The name of the attribute that holds the authorization code.
pub const AUTHORIZATION_CODE_ATTRIBUTE: &str = "authorization_code";
/// The name of the attribute that holds the proof key of the client.
pub const CODE_CHALLENGE_ATTRIBUTE: &str = "code_challenge";
/// The name of the attribute that holds the method of the proof key.
pub const CODE_CHALLENGE_METHOD_ATTRIBUTE: &str = "code_challenge_method";
/// The name of the attribute that holds the redirect URI of the request.
pub const REDIRECT_URI_ATTRIBUTE: &str = "redirect_uri";
/// The name of the attribute that holds the state of the request.
pub const STATE_ATTRIBUTE: &str = "state";
/// The name of the attribute that holds the scopes of the request.
pub const SCOPES_ATTRIBUTE: &str = "scopes";
/// The name of the attribute that holds the nonce of the request.
pub const NONCE_ATTRIBUTE: &str = "nonce";

/// An authorization the authorization server granted, and the tokens it issued
/// for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2Authorization {
    id: String,
    registered_client_id: String,
    principal_name: String,
    grant_type: Option<AuthorizationGrantType>,
    attributes: HashMap<String, String>,
    access_token: Option<OAuth2AccessToken>,
    refresh_token: Option<OAuth2RefreshToken>,
}

impl OAuth2Authorization {
    /// Creates an authorization.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the authorization.
    /// * `registered_client_id` - The identifier of the client the
    ///   authorization was granted to.
    /// * `principal_name` - The name of the principal the authorization was
    ///   granted for.
    pub fn new(
        id: impl Into<String>,
        registered_client_id: impl Into<String>,
        principal_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            registered_client_id: registered_client_id.into(),
            principal_name: principal_name.into(),
            grant_type: None,
            attributes: HashMap::new(),
            access_token: None,
            refresh_token: None,
        }
    }

    /// Sets the grant of the authorization.
    ///
    /// # Arguments
    ///
    /// * `grant_type` - The grant of the authorization.
    pub fn set_grant_type(mut self, grant_type: AuthorizationGrantType) -> Self {
        self.grant_type = Some(grant_type);
        self
    }

    /// Sets an attribute of the authorization.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attribute.
    /// * `value` - The value of the attribute.
    pub fn set_attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(name.into(), value.into());
        self
    }

    /// Sets the access token of the authorization.
    ///
    /// # Arguments
    ///
    /// * `access_token` - The access token.
    pub fn access_token(mut self, access_token: OAuth2AccessToken) -> Self {
        self.access_token = Some(access_token);
        self
    }

    /// Sets the refresh token of the authorization.
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token.
    pub fn refresh_token(mut self, refresh_token: OAuth2RefreshToken) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    /// Returns the identifier of the authorization.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the identifier of the client the authorization was granted to.
    pub fn registered_client_id(&self) -> &str {
        &self.registered_client_id
    }

    /// Returns the name of the principal the authorization was granted for.
    pub fn principal_name(&self) -> &str {
        &self.principal_name
    }

    /// Returns the grant of the authorization.
    pub fn grant_type(&self) -> Option<AuthorizationGrantType> {
        self.grant_type
    }

    /// Returns an attribute of the authorization.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attribute.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    /// Returns the access token of the authorization, if one was issued.
    pub fn access_token_value(&self) -> Option<&OAuth2AccessToken> {
        self.access_token.as_ref()
    }

    /// Returns the refresh token of the authorization, if one was issued.
    pub fn refresh_token_value(&self) -> Option<&OAuth2RefreshToken> {
        self.refresh_token.as_ref()
    }

    /// Sets the access token of the authorization.
    ///
    /// # Arguments
    ///
    /// * `access_token` - The access token.
    pub fn set_access_token(&mut self, access_token: OAuth2AccessToken) {
        self.access_token = Some(access_token);
    }

    /// Sets the refresh token of the authorization.
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token.
    pub fn set_refresh_token(&mut self, refresh_token: OAuth2RefreshToken) {
        self.refresh_token = Some(refresh_token);
    }

    /// Removes the access token of the authorization.
    ///
    /// The authorization stays stored, so a refresh token of it can still
    /// issued a new access token.
    pub fn remove_access_token(&mut self) {
        self.access_token = None;
    }

    /// Returns the scopes of the authorization.
    pub fn scopes(&self) -> Vec<String> {
        self.attribute(SCOPES_ATTRIBUTE)
            .map(|scopes| scopes.split_whitespace().map(ToString::to_string).collect())
            .unwrap_or_default()
    }

    /// Returns whether the access token of the authorization is still valid.
    ///
    /// # Arguments
    ///
    /// * `now` - The time to compare with, in seconds since the epoch.
    pub fn is_access_token_active(&self, now: u64) -> bool {
        self.access_token
            .as_ref()
            .is_some_and(|token| !token.is_expired(now))
    }

    /// Returns whether the refresh token of the authorization is still valid.
    ///
    /// # Arguments
    ///
    /// * `now` - The time to compare with, in seconds since the epoch.
    pub fn is_refresh_token_active(&self, now: u64) -> bool {
        self.refresh_token
            .as_ref()
            .is_some_and(|token| !token.is_expired(now))
    }
}

/// Stores the authorizations the authorization server granted.
pub trait OAuth2AuthorizationService: Send + Sync {
    /// Stores an authorization, replacing the one with the same id.
    ///
    /// # Arguments
    ///
    /// * `authorization` - The authorization to store.
    fn save(&self, authorization: OAuth2Authorization);

    /// Removes an authorization.
    ///
    /// # Arguments
    ///
    /// * `authorization` - The authorization to remove.
    fn remove(&self, authorization: &OAuth2Authorization);

    /// Returns the authorization with the given id, if one is stored.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the authorization.
    fn find_by_id(&self, id: &str) -> Option<OAuth2Authorization>;

    /// Returns the authorization that was granted for the given token.
    ///
    /// # Arguments
    ///
    /// * `token` - The value of the token.
    /// * `token_type` - The type of the token, or `None` when any type matches.
    fn find_by_token(
        &self,
        token: &str,
        token_type: Option<OAuth2TokenType>,
    ) -> Option<OAuth2Authorization>;
}

/// An [`OAuth2AuthorizationService`] that keeps its authorizations in memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryOAuth2AuthorizationService {
    authorizations: Arc<RwLock<HashMap<String, OAuth2Authorization>>>,
}

impl OAuth2AuthorizationService for InMemoryOAuth2AuthorizationService {
    fn save(&self, authorization: OAuth2Authorization) {
        self.authorizations
            .write()
            .expect("the authorizations lock is not poisoned")
            .insert(authorization.id().to_string(), authorization);
    }

    fn remove(&self, authorization: &OAuth2Authorization) {
        self.authorizations
            .write()
            .expect("the authorizations lock is not poisoned")
            .remove(authorization.id());
    }

    fn find_by_id(&self, id: &str) -> Option<OAuth2Authorization> {
        self.authorizations
            .read()
            .expect("the authorizations lock is not poisoned")
            .get(id)
            .cloned()
    }

    fn find_by_token(
        &self,
        token: &str,
        token_type: Option<OAuth2TokenType>,
    ) -> Option<OAuth2Authorization> {
        self.authorizations
            .read()
            .expect("the authorizations lock is not poisoned")
            .values()
            .find(|authorization| {
                let matches_access = token_type
                    .map(|token_type| token_type == OAuth2TokenType::AccessToken)
                    .unwrap_or(true)
                    && authorization
                        .access_token_value()
                        .is_some_and(|access_token| access_token.token_value() == token);
                let matches_refresh = token_type
                    .map(|token_type| token_type == OAuth2TokenType::RefreshToken)
                    .unwrap_or(true)
                    && authorization
                        .refresh_token_value()
                        .is_some_and(|refresh_token| refresh_token.token_value() == token);
                let matches_code =
                    authorization.attribute(AUTHORIZATION_CODE_ATTRIBUTE) == Some(token);
                let matches_state = authorization.attribute(STATE_ATTRIBUTE) == Some(token)
                    && token_type
                        .map(|token_type| token_type == OAuth2TokenType::State)
                        .unwrap_or(true);

                matches_access || matches_refresh || matches_code || matches_state
            })
            .cloned()
    }
}

/// Returns whether the authorization code of the authorization is still valid,
/// and consumes it so that it cannot be used twice.
///
/// # Arguments
///
/// * `authorization` - The authorization to consume the code of.
/// * `code` - The authorization code of the token request.
pub fn consume_authorization_code(
    authorization: &mut OAuth2Authorization,
    code: &str,
    code_time_to_live: u64,
) -> Result<(), String> {
    match authorization.attribute(AUTHORIZATION_CODE_ATTRIBUTE) {
        Some(candidate) if candidate == code => {}
        _ => return Err("The authorization code is invalid".to_string()),
    }

    let issued_at = authorization
        .attribute("authorization_code_issued_at")
        .and_then(|issued_at| issued_at.parse::<u64>().ok())
        .unwrap_or_else(now);
    if now() >= issued_at + code_time_to_live {
        return Err("The authorization code is expired".to_string());
    }

    authorization
        .attributes
        .remove(AUTHORIZATION_CODE_ATTRIBUTE);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        consume_authorization_code, InMemoryOAuth2AuthorizationService, OAuth2Authorization,
        OAuth2AuthorizationService, AUTHORIZATION_CODE_ATTRIBUTE, SCOPES_ATTRIBUTE,
    };
    use crate::oauth2::server::authorization::token::{now, OAuth2AccessToken, OAuth2TokenType};

    fn authorization() -> OAuth2Authorization {
        OAuth2Authorization::new("authorization-id", "demo-client", "user")
            .set_attribute(AUTHORIZATION_CODE_ATTRIBUTE, "the-code")
            .set_attribute("authorization_code_issued_at", now().to_string())
            .set_attribute(SCOPES_ATTRIBUTE, "read write")
            .access_token(OAuth2AccessToken::new(
                "the-access-token",
                now(),
                now() + 300,
                vec!["read".to_string()],
            ))
    }

    #[test]
    fn a_service_finds_an_authorization_by_id_and_by_token() {
        let service = InMemoryOAuth2AuthorizationService::default();
        service.save(authorization());

        assert!(service.find_by_id("authorization-id").is_some());
        assert!(service
            .find_by_token("the-access-token", Some(OAuth2TokenType::AccessToken))
            .is_some());
        assert!(service
            .find_by_token("the-access-token", Some(OAuth2TokenType::RefreshToken))
            .is_none());
        assert!(service.find_by_token("unknown", None).is_none());
    }

    #[test]
    fn the_authorization_carries_its_scopes_and_tokens() {
        let authorization = authorization();

        assert_eq!(authorization.scopes(), ["read", "write"]);
        assert!(authorization.is_access_token_active(now()));
        assert!(!authorization.is_refresh_token_active(now()));
    }

    #[test]
    fn an_authorization_code_is_consumed_once() {
        let mut authorization = authorization();

        assert_eq!(
            consume_authorization_code(&mut authorization, "the-code", 300),
            Ok(())
        );
        assert!(authorization
            .attribute(AUTHORIZATION_CODE_ATTRIBUTE)
            .is_none());
        assert!(consume_authorization_code(&mut authorization, "the-code", 300).is_err());
    }

    #[test]
    fn an_authorization_code_that_does_not_match_is_reported() {
        let mut authorization = authorization();

        assert_eq!(
            consume_authorization_code(&mut authorization, "other", 300),
            Err("The authorization code is invalid".to_string())
        );
    }
}
