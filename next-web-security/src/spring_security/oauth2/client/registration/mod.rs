//! The client registrations known to the application.

use crate::oauth2::AuthorizationGrantType;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRegistration {
    registration_id: String,
    client_name: String,
    client_id: String,
    authorization_uri: String,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    authorization_grant_type: AuthorizationGrantType,
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
            authorization_uri,
            redirect_uri: None,
            scopes: Vec::new(),
            authorization_grant_type: AuthorizationGrantType::AuthorizationCode,
        }
    }

    pub fn set_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
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
