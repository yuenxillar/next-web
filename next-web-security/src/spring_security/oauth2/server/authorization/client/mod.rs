//! The clients that are registered with the authorization server.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use super::error::OAuth2Error;

/// The way a client authenticates at the token endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ClientAuthenticationMethod {
    /// The client sends its credentials with the `Authorization` header.
    ClientSecretBasic,
    /// The client sends its credentials as form parameters.
    ClientSecretPost,
    /// The client does not authenticate, which is what a public client does.
    None,
}

impl ClientAuthenticationMethod {
    /// Returns the value of the method in the specifications.
    pub fn value(&self) -> &'static str {
        match self {
            Self::ClientSecretBasic => "client_secret_basic",
            Self::ClientSecretPost => "client_secret_post",
            Self::None => "none",
        }
    }
}

/// The grant a client is allowed to use.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AuthorizationGrantType {
    /// The authorization code grant.
    AuthorizationCode,
    /// The client credentials grant.
    ClientCredentials,
    /// The refresh token grant.
    RefreshToken,
}

impl AuthorizationGrantType {
    /// Returns the value of the grant in the specifications.
    pub fn value(&self) -> &'static str {
        match self {
            Self::AuthorizationCode => "authorization_code",
            Self::ClientCredentials => "client_credentials",
            Self::RefreshToken => "refresh_token",
        }
    }

    /// Returns the grant of the given value, if the value names one.
    ///
    /// # Arguments
    ///
    /// * `value` - The value of the grant.
    pub fn from_value(value: &str) -> Option<Self> {
        match value {
            "authorization_code" => Some(Self::AuthorizationCode),
            "client_credentials" => Some(Self::ClientCredentials),
            "refresh_token" => Some(Self::RefreshToken),
            _ => None,
        }
    }
}

/// The settings of a registered client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientSettings {
    require_proof_key: bool,
    require_authorization_consent: bool,
}

impl ClientSettings {
    /// Creates a builder for the settings.
    pub fn builder() -> ClientSettingsBuilder {
        ClientSettingsBuilder::default()
    }

    /// Returns whether the client has to use a proof key for the authorization
    /// code grant.
    pub fn require_proof_key(&self) -> bool {
        self.require_proof_key
    }

    /// Returns whether the client has to ask the resource owner for consent.
    pub fn require_authorization_consent(&self) -> bool {
        self.require_authorization_consent
    }
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            require_proof_key: true,
            require_authorization_consent: true,
        }
    }
}

/// Builds the settings of a registered client.
#[derive(Clone, Debug, Default)]
pub struct ClientSettingsBuilder {
    settings: ClientSettings,
}

impl ClientSettingsBuilder {
    /// Sets whether the client has to use a proof key.
    ///
    /// # Arguments
    ///
    /// * `require_proof_key` - Whether a proof key is required.
    pub fn require_proof_key(mut self, require_proof_key: bool) -> Self {
        self.settings.require_proof_key = require_proof_key;
        self
    }

    /// Sets whether the client has to ask the resource owner for consent.
    ///
    /// # Arguments
    ///
    /// * `require_authorization_consent` - Whether consent is required.
    pub fn require_authorization_consent(mut self, require_authorization_consent: bool) -> Self {
        self.settings.require_authorization_consent = require_authorization_consent;
        self
    }

    /// Builds the settings.
    pub fn build(self) -> ClientSettings {
        self.settings
    }
}

/// The settings of the tokens issued to a registered client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenSettings {
    access_token_time_to_live: u64,
    refresh_token_time_to_live: u64,
    reuse_refresh_tokens: bool,
}

impl TokenSettings {
    /// Creates a builder for the settings.
    pub fn builder() -> TokenSettingsBuilder {
        TokenSettingsBuilder::default()
    }

    /// Returns the lifetime of an access token, in seconds.
    pub fn access_token_time_to_live(&self) -> u64 {
        self.access_token_time_to_live
    }

    /// Returns the lifetime of a refresh token, in seconds.
    pub fn refresh_token_time_to_live(&self) -> u64 {
        self.refresh_token_time_to_live
    }

    /// Returns whether the refresh token is reused when it is rotated.
    pub fn reuse_refresh_tokens(&self) -> bool {
        self.reuse_refresh_tokens
    }
}

impl Default for TokenSettings {
    fn default() -> Self {
        Self {
            access_token_time_to_live: 300,
            refresh_token_time_to_live: 3600,
            reuse_refresh_tokens: true,
        }
    }
}

/// Builds the settings of the tokens issued to a registered client.
#[derive(Clone, Debug, Default)]
pub struct TokenSettingsBuilder {
    settings: TokenSettings,
}

impl TokenSettingsBuilder {
    /// Sets the lifetime of an access token, in seconds.
    ///
    /// # Arguments
    ///
    /// * `access_token_time_to_live` - The lifetime in seconds.
    pub fn access_token_time_to_live(mut self, access_token_time_to_live: u64) -> Self {
        self.settings.access_token_time_to_live = access_token_time_to_live;
        self
    }

    /// Sets the lifetime of a refresh token, in seconds.
    ///
    /// # Arguments
    ///
    /// * `refresh_token_time_to_live` - The lifetime in seconds.
    pub fn refresh_token_time_to_live(mut self, refresh_token_time_to_live: u64) -> Self {
        self.settings.refresh_token_time_to_live = refresh_token_time_to_live;
        self
    }

    /// Sets whether the refresh token is reused when it is rotated.
    ///
    /// # Arguments
    ///
    /// * `reuse_refresh_tokens` - Whether the refresh token is reused.
    pub fn reuse_refresh_tokens(mut self, reuse_refresh_tokens: bool) -> Self {
        self.settings.reuse_refresh_tokens = reuse_refresh_tokens;
        self
    }

    /// Builds the settings.
    pub fn build(self) -> TokenSettings {
        self.settings
    }
}

/// A client that is registered with the authorization server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredClient {
    id: String,
    client_id: String,
    client_secret: Option<String>,
    client_name: Option<String>,
    client_authentication_methods: Vec<ClientAuthenticationMethod>,
    authorization_grant_types: Vec<AuthorizationGrantType>,
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
    client_settings: ClientSettings,
    token_settings: TokenSettings,
}

impl RegisteredClient {
    /// Creates a registration for the given client id.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The identifier of the client.
    pub fn with_client_id(client_id: impl Into<String>) -> Self {
        let client_id = client_id.into();
        assert!(!client_id.trim().is_empty(), "client_id cannot be empty");
        Self {
            id: client_id.clone(),
            client_id,
            client_secret: None,
            client_name: None,
            client_authentication_methods: vec![ClientAuthenticationMethod::ClientSecretBasic],
            authorization_grant_types: vec![AuthorizationGrantType::AuthorizationCode],
            redirect_uris: Vec::new(),
            scopes: Vec::new(),
            client_settings: ClientSettings::default(),
            token_settings: TokenSettings::default(),
        }
    }

    /// Sets the identifier of the registration.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the registration.
    pub fn set_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the secret of the client.
    ///
    /// The secret is stored as it is given; a value that starts with `{noop}`
    /// is compared as it is, and every other value is matched with the password
    /// encoder of the application when one is configured.
    ///
    /// # Arguments
    ///
    /// * `client_secret` - The secret of the client.
    pub fn set_client_secret(mut self, client_secret: impl Into<String>) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Sets the name of the client.
    ///
    /// # Arguments
    ///
    /// * `client_name` - The name of the client.
    pub fn set_client_name(mut self, client_name: impl Into<String>) -> Self {
        self.client_name = Some(client_name.into());
        self
    }

    /// Adds the way the client authenticates.
    ///
    /// # Arguments
    ///
    /// * `client_authentication_method` - The method of the client.
    pub fn client_authentication_method(
        mut self,
        client_authentication_method: ClientAuthenticationMethod,
    ) -> Self {
        if !self
            .client_authentication_methods
            .contains(&client_authentication_method)
        {
            self.client_authentication_methods
                .push(client_authentication_method);
        }
        self
    }

    /// Adds the grants the client is allowed to use.
    ///
    /// # Arguments
    ///
    /// * `authorization_grant_type` - The grant of the client.
    pub fn authorization_grant_type(
        mut self,
        authorization_grant_type: AuthorizationGrantType,
    ) -> Self {
        if !self
            .authorization_grant_types
            .contains(&authorization_grant_type)
        {
            self.authorization_grant_types
                .push(authorization_grant_type);
        }
        self
    }

    /// Adds a redirect URI of the client.
    ///
    /// # Arguments
    ///
    /// * `redirect_uri` - The redirect URI of the client.
    pub fn redirect_uri(mut self, redirect_uri: impl Into<String>) -> Self {
        self.redirect_uris.push(redirect_uri.into());
        self
    }

    /// Adds a scope of the client.
    ///
    /// # Arguments
    ///
    /// * `scope` - The scope of the client.
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Sets the settings of the client.
    ///
    /// # Arguments
    ///
    /// * `client_settings` - The settings of the client.
    pub fn set_client_settings(mut self, client_settings: ClientSettings) -> Self {
        self.client_settings = client_settings;
        self
    }

    /// Sets the token settings of the client.
    ///
    /// # Arguments
    ///
    /// * `token_settings` - The token settings of the client.
    pub fn set_token_settings(mut self, token_settings: TokenSettings) -> Self {
        self.token_settings = token_settings;
        self
    }

    /// Returns the identifier of the registration.
    pub fn id_value(&self) -> &str {
        &self.id
    }

    /// Returns the identifier of the client.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Returns the secret of the client, if one is set.
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// Returns the name of the client, if one is set.
    pub fn client_name(&self) -> Option<&str> {
        self.client_name.as_deref()
    }

    /// Returns the ways the client authenticates.
    pub fn client_authentication_methods(&self) -> &[ClientAuthenticationMethod] {
        &self.client_authentication_methods
    }

    /// Returns the grants the client is allowed to use.
    pub fn authorization_grant_types(&self) -> &[AuthorizationGrantType] {
        &self.authorization_grant_types
    }

    /// Returns the redirect URIs of the client.
    pub fn redirect_uris(&self) -> &[String] {
        &self.redirect_uris
    }

    /// Returns the scopes of the client.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns the settings of the client.
    pub fn client_settings(&self) -> &ClientSettings {
        &self.client_settings
    }

    /// Returns the token settings of the client.
    pub fn token_settings(&self) -> &TokenSettings {
        &self.token_settings
    }

    /// Returns whether the client is allowed to use the given grant.
    ///
    /// # Arguments
    ///
    /// * `grant_type` - The grant to look up.
    pub fn is_authorized_grant_type(&self, grant_type: AuthorizationGrantType) -> bool {
        self.authorization_grant_types.contains(&grant_type)
    }

    /// Returns whether the given redirect URI is registered for the client.
    ///
    /// # Arguments
    ///
    /// * `redirect_uri` - The redirect URI to look up.
    pub fn has_redirect_uri(&self, redirect_uri: &str) -> bool {
        self.redirect_uris
            .iter()
            .any(|registered| registered == redirect_uri)
    }

    /// Returns whether the client is allowed to use the given scope.
    ///
    /// # Arguments
    ///
    /// * `scope` - The scope to look up.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|registered| registered == scope)
    }
}

/// Stores the clients that are registered with the authorization server.
pub trait RegisteredClientRepository: Send + Sync {
    /// Returns the registration with the given id, if one is stored.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the registration.
    fn find_by_id(&self, id: &str) -> Option<RegisteredClient>;

    /// Returns the registration of the given client, if one is stored.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The identifier of the client.
    fn find_by_client_id(&self, client_id: &str) -> Option<RegisteredClient>;

    /// Stores a registration, replacing the one with the same id.
    ///
    /// # Arguments
    ///
    /// * `registered_client` - The registration to store.
    fn save(&self, registered_client: RegisteredClient);
}

/// A [`RegisteredClientRepository`] that keeps its registrations in memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryRegisteredClientRepository {
    clients: Arc<RwLock<HashMap<String, RegisteredClient>>>,
}

impl InMemoryRegisteredClientRepository {
    /// Creates a repository with the given registrations.
    ///
    /// # Arguments
    ///
    /// * `registered_clients` - The registrations of the repository.
    pub fn new<I>(registered_clients: I) -> Self
    where
        I: IntoIterator<Item = RegisteredClient>,
    {
        let clients = registered_clients
            .into_iter()
            .map(|client| (client.client_id().to_string(), client))
            .collect();
        Self {
            clients: Arc::new(RwLock::new(clients)),
        }
    }
}

impl RegisteredClientRepository for InMemoryRegisteredClientRepository {
    fn find_by_id(&self, id: &str) -> Option<RegisteredClient> {
        self.clients
            .read()
            .expect("the registered clients lock is not poisoned")
            .values()
            .find(|client| client.id_value() == id)
            .cloned()
    }

    fn find_by_client_id(&self, client_id: &str) -> Option<RegisteredClient> {
        self.clients
            .read()
            .expect("the registered clients lock is not poisoned")
            .get(client_id)
            .cloned()
    }

    fn save(&self, registered_client: RegisteredClient) {
        self.clients
            .write()
            .expect("the registered clients lock is not poisoned")
            .insert(registered_client.client_id().to_string(), registered_client);
    }
}

/// Matches the secret a client presents against the stored one.
///
/// A stored secret that starts with `{noop}` is compared as it is; every other
/// secret is matched with the password encoder of the application when one is
/// configured, and is compared as it is otherwise.
///
/// # Arguments
///
/// * `presented` - The secret the client presented.
/// * `stored` - The secret of the registration.
/// * `password_encoder` - The encoder of the application, if one is configured.
pub fn client_secret_matches(
    presented: &str,
    stored: &str,
    password_encoder: Option<&Arc<dyn crate::crypto::password::PasswordEncoder>>,
) -> bool {
    if let Some(noop) = stored.strip_prefix("{noop}") {
        return noop == presented;
    }

    match password_encoder {
        Some(password_encoder) => password_encoder.matches(presented, stored),
        None => stored == presented,
    }
}

/// Returns the error a client that is not stored reports.
///
/// # Arguments
///
/// * `client_id` - The identifier of the client.
pub fn client_not_found(client_id: &str) -> OAuth2Error {
    OAuth2Error::invalid_client(format!("Registered client not found: {client_id}"))
}

#[cfg(test)]
mod tests {
    use super::{
        client_secret_matches, AuthorizationGrantType, ClientAuthenticationMethod, ClientSettings,
        InMemoryRegisteredClientRepository, RegisteredClient, RegisteredClientRepository,
        TokenSettings,
    };

    fn client() -> RegisteredClient {
        RegisteredClient::with_client_id("demo-client")
            .set_client_secret("{noop}secret")
            .set_client_name("Demo Client")
            .client_authentication_method(ClientAuthenticationMethod::ClientSecretBasic)
            .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
            .authorization_grant_type(AuthorizationGrantType::RefreshToken)
            .redirect_uri("https://client.example/callback")
            .scope("read")
            .set_client_settings(ClientSettings::builder().require_proof_key(false).build())
            .set_token_settings(
                TokenSettings::builder()
                    .access_token_time_to_live(60)
                    .build(),
            )
    }

    #[test]
    fn a_registration_carries_the_client_settings() {
        let client = client();

        assert_eq!(client.client_id(), "demo-client");
        assert_eq!(client.client_settings().require_proof_key(), false);
        assert_eq!(client.token_settings().access_token_time_to_live(), 60);
        assert!(client.is_authorized_grant_type(AuthorizationGrantType::RefreshToken));
        assert!(!client.is_authorized_grant_type(AuthorizationGrantType::ClientCredentials));
        assert!(client.has_redirect_uri("https://client.example/callback"));
        assert!(client.has_scope("read"));
        assert!(!client.has_scope("write"));
    }

    #[test]
    fn a_repository_finds_the_registrations_it_stores() {
        let repository = InMemoryRegisteredClientRepository::new([client()]);

        assert!(repository.find_by_client_id("demo-client").is_some());
        assert!(repository.find_by_id("demo-client").is_some());
        assert!(repository.find_by_client_id("other").is_none());
    }

    #[test]
    fn a_repository_stores_a_registration_that_replaces_the_previous_one() {
        let repository = InMemoryRegisteredClientRepository::default();
        repository.save(client());
        repository.save(client().set_client_name("Renamed"));

        let stored = repository
            .find_by_client_id("demo-client")
            .expect("the registration is stored");

        assert_eq!(stored.client_name(), Some("Renamed"));
    }

    #[test]
    fn a_noop_secret_is_compared_as_it_is() {
        assert!(client_secret_matches("secret", "{noop}secret", None));
        assert!(!client_secret_matches("other", "{noop}secret", None));
    }

    #[test]
    fn a_secret_without_an_encoder_is_compared_as_it_is() {
        assert!(client_secret_matches("secret", "secret", None));
        assert!(!client_secret_matches("other", "secret", None));
    }
}
