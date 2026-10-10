//! The client registrations known to the application.

use crate::oauth2::AuthorizationGrantType;

/// The way a client authenticates at the token endpoint of a provider.
///
/// The methods are those of the specification. A public client does not
/// authenticate at all, which is why it has to use a proof key for the
/// authorization code grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ClientAuthenticationMethod {
    /// The client sends its credentials with the `Authorization` header.
    ClientSecretBasic,
    /// The client sends its credentials as form parameters.
    ClientSecretPost,
    /// The client does not authenticate.
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRegistration {
    registration_id: String,
    client_name: String,
    client_id: String,
    client_secret: Option<String>,
    authorization_uri: String,
    token_uri: Option<String>,
    user_info_uri: Option<String>,
    issuer_uri: Option<String>,
    jwk_set_uri: Option<String>,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    authorization_grant_type: AuthorizationGrantType,
    client_authentication_method: ClientAuthenticationMethod,
    require_proof_key: bool,
}

impl ClientRegistration {
    pub fn new(
        registration_id: impl Into<String>,
        client_name: impl Into<String>,
        authorization_uri: impl Into<String>,
    ) -> Self {
        let registration_id = registration_id.into();
        let client_name = client_name.into();
        let authorization_uri = authorization_uri.into();
        assert!(
            !registration_id.trim().is_empty(),
            "registration_id cannot be empty"
        );
        assert!(
            !client_name.trim().is_empty(),
            "client_name cannot be empty"
        );
        assert!(
            !authorization_uri.trim().is_empty(),
            "authorization_uri cannot be empty"
        );
        Self {
            registration_id,
            client_name,
            client_id: String::new(),
            client_secret: None,
            authorization_uri,
            token_uri: None,
            user_info_uri: None,
            issuer_uri: None,
            jwk_set_uri: None,
            redirect_uri: None,
            scopes: Vec::new(),
            authorization_grant_type: AuthorizationGrantType::AuthorizationCode,
            client_authentication_method: ClientAuthenticationMethod::ClientSecretBasic,
            require_proof_key: false,
        }
    }

    pub fn set_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    /// Sets the secret of the client.
    ///
    /// # Arguments
    ///
    /// * `client_secret` - The secret of the client.
    pub fn set_client_secret(mut self, client_secret: impl Into<String>) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Sets the URI of the token endpoint of the provider.
    ///
    /// # Arguments
    ///
    /// * `token_uri` - The URI of the token endpoint.
    pub fn set_token_uri(mut self, token_uri: impl Into<String>) -> Self {
        self.token_uri = Some(token_uri.into());
        self
    }

    /// Sets the URI of the user info endpoint of the provider.
    ///
    /// # Arguments
    ///
    /// * `user_info_uri` - The URI of the user info endpoint.
    pub fn set_user_info_uri(mut self, user_info_uri: impl Into<String>) -> Self {
        self.user_info_uri = Some(user_info_uri.into());
        self
    }

    /// Sets the issuer of the provider.
    ///
    /// The issuer is the value the `iss` claim of an identifier token has to
    /// name.
    ///
    /// # Arguments
    ///
    /// * `issuer_uri` - The identifier of the provider.
    pub fn set_issuer_uri(mut self, issuer_uri: impl Into<String>) -> Self {
        self.issuer_uri = Some(issuer_uri.into());
        self
    }

    /// Sets the URI of the JWK set of the provider.
    ///
    /// The keys of the set verify the signature of an identifier token. A
    /// client that does not name the set cannot verify the tokens of the
    /// provider, so it refuses them.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_uri` - The URI of the JWK set of the provider.
    pub fn set_jwk_set_uri(mut self, jwk_set_uri: impl Into<String>) -> Self {
        self.jwk_set_uri = Some(jwk_set_uri.into());
        self
    }

    /// Sets the way the client authenticates at the token endpoint.
    ///
    /// # Arguments
    ///
    /// * `client_authentication_method` - The method of the client.
    pub fn set_client_authentication_method(
        mut self,
        client_authentication_method: ClientAuthenticationMethod,
    ) -> Self {
        self.client_authentication_method = client_authentication_method;
        self
    }

    /// Sets whether the client uses a proof key for the authorization code
    /// grant.
    ///
    /// # Arguments
    ///
    /// * `require_proof_key` - Whether a proof key is used.
    pub fn set_require_proof_key(mut self, require_proof_key: bool) -> Self {
        self.require_proof_key = require_proof_key;
        self
    }

    /// Returns the secret of the client, if one is set.
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// Returns the URI of the token endpoint of the provider, if one is set.
    pub fn token_uri(&self) -> Option<&str> {
        self.token_uri.as_deref()
    }

    /// Returns the URI of the user info endpoint of the provider, if one is set.
    pub fn user_info_uri(&self) -> Option<&str> {
        self.user_info_uri.as_deref()
    }

    /// Returns the issuer of the provider, if one is set.
    pub fn issuer_uri(&self) -> Option<&str> {
        self.issuer_uri.as_deref()
    }

    /// Returns the URI of the JWK set of the provider, if one is set.
    pub fn jwk_set_uri(&self) -> Option<&str> {
        self.jwk_set_uri.as_deref()
    }

    /// Returns the way the client authenticates at the token endpoint.
    pub fn client_authentication_method(&self) -> ClientAuthenticationMethod {
        self.client_authentication_method
    }

    /// Returns whether the client uses a proof key.
    pub fn require_proof_key(&self) -> bool {
        self.require_proof_key
    }

    pub fn set_redirect_uri(mut self, redirect_uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(redirect_uri.into());
        self
    }

    pub fn set_scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    pub fn set_authorization_grant_type(
        mut self,
        authorization_grant_type: AuthorizationGrantType,
    ) -> Self {
        self.authorization_grant_type = authorization_grant_type;
        self
    }

    pub fn registration_id(&self) -> &str {
        &self.registration_id
    }

    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn authorization_uri(&self) -> &str {
        &self.authorization_uri
    }

    pub fn redirect_uri(&self) -> Option<&str> {
        self.redirect_uri.as_deref()
    }

    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn authorization_grant_type(&self) -> &AuthorizationGrantType {
        &self.authorization_grant_type
    }

    pub fn is_authorization_code(&self) -> bool {
        self.authorization_grant_type == AuthorizationGrantType::AuthorizationCode
    }
}

pub trait ClientRegistrationRepository: Send + Sync {
    fn find_by_registration_id(&self, registration_id: &str) -> Option<ClientRegistration>;

    fn client_registrations(&self) -> Vec<ClientRegistration> {
        Vec::new()
    }
}

#[derive(Clone, Default)]
pub struct InMemoryClientRegistrationRepository {
    registrations: Vec<ClientRegistration>,
}

impl InMemoryClientRegistrationRepository {
    pub fn new(registrations: impl IntoIterator<Item = ClientRegistration>) -> Self {
        Self {
            registrations: registrations.into_iter().collect(),
        }
    }
}

impl ClientRegistrationRepository for InMemoryClientRegistrationRepository {
    fn find_by_registration_id(&self, registration_id: &str) -> Option<ClientRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.registration_id() == registration_id)
            .cloned()
    }

    fn client_registrations(&self) -> Vec<ClientRegistration> {
        self.registrations.clone()
    }
}
