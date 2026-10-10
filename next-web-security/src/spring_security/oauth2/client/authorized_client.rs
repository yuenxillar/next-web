//! The authorized client manager and the providers it delegates to.
//!
//! An application authorizes an OAuth 2.0 client through the
//! [`OAuth2AuthorizedClientManager`] of its application context. The manager
//! loads the authorized client the request already has, or the client
//! registration the request names, and hands the state to the
//! [`OAuth2AuthorizedClientProvider`] it was configured with. The default
//! provider is a [`DelegatingOAuth2AuthorizedClientProvider`] that holds the
//! providers of the supported grants in order.

use std::{collections::HashMap, fmt, sync::Arc};

use next_web_core::{
    any::AnyValue,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{Authentication, AuthenticationError, AuthenticationErrorKind},
    oauth2::{
        AuthorizationGrantType, ClientAuthorizationRequiredError, ClientRegistration,
        ClientRegistrationRepository, OAuth2AccessToken, OAuth2AuthorizedClientRepository,
        OAuth2RefreshToken,
    },
    web::authentication::Identity,
};

/// The name of the attribute that holds the scopes the request asks for.
///
/// The attribute is mapped from the `scope` parameter of the request by
/// [`DefaultOAuth2AuthorizedClientManager`].
pub const REQUEST_SCOPE_ATTRIBUTE_NAME: &str =
    "next.security.oauth2.client.OAuth2AuthorizationContext.REQUEST_SCOPE";

/// The error an [`OAuth2AuthorizedClientProvider`] reports.
#[derive(Debug)]
pub enum OAuth2AuthorizeError {
    /// The client has to obtain authorization from the resource owner first.
    ///
    /// The error is what the filter that starts an authorization request
    /// reacts to, so it is reported with the registration it names.
    AuthorizationRequired(ClientAuthorizationRequiredError),

    /// The authorization attempt failed.
    Authentication(AuthenticationError),
}

impl fmt::Display for OAuth2AuthorizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorizationRequired(error) => write!(f, "{error}"),
            Self::Authentication(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for OAuth2AuthorizeError {}

impl From<ClientAuthorizationRequiredError> for OAuth2AuthorizeError {
    fn from(error: ClientAuthorizationRequiredError) -> Self {
        Self::AuthorizationRequired(error)
    }
}

impl From<AuthenticationError> for OAuth2AuthorizeError {
    fn from(error: AuthenticationError) -> Self {
        Self::Authentication(error)
    }
}

/// A request to authorize (or re-authorize) an OAuth 2.0 client.
#[derive(Clone)]
pub struct OAuth2AuthorizeRequest {
    client_registration_id: String,
    principal: Arc<dyn Authentication>,
    attributes: HashMap<String, AnyValue>,
    authorized_client: Option<OAuth2AuthorizedClient>,
}

impl OAuth2AuthorizeRequest {
    /// Creates a request that authorizes the registration
    /// `client_registration_id` for `principal`.
    ///
    /// # Arguments
    ///
    /// * `client_registration_id` - The id of the client registration.
    /// * `principal` - The principal the authorized client belongs to.
    pub fn with_client_registration(
        client_registration_id: impl Into<String>,
        principal: Arc<dyn Authentication>,
    ) -> Self {
        Self {
            client_registration_id: client_registration_id.into(),
            principal,
            attributes: HashMap::new(),
            authorized_client: None,
        }
    }

    /// Creates a request that re-authorizes `authorized_client`.
    ///
    /// # Arguments
    ///
    /// * `authorized_client` - The client to re-authorize.
    /// * `principal` - The principal the authorized client belongs to.
    pub fn with_authorized_client(
        authorized_client: OAuth2AuthorizedClient,
        principal: Arc<dyn Authentication>,
    ) -> Self {
        Self {
            client_registration_id: authorized_client
                .client_registration()
                .registration_id()
                .to_string(),
            principal,
            attributes: HashMap::new(),
            authorized_client: Some(authorized_client),
        }
    }

    /// Returns the id of the client registration of the request.
    pub fn client_registration_id(&self) -> &str {
        &self.client_registration_id
    }

    /// Returns the principal the authorized client belongs to.
    pub fn principal(&self) -> &Arc<dyn Authentication> {
        &self.principal
    }

    /// Returns the attributes of the request.
    pub fn attributes(&self) -> &HashMap<String, AnyValue> {
        &self.attributes
    }

    /// Returns the authorized client the request carries, if it re-authorizes
    /// one.
    pub fn authorized_client(&self) -> Option<&OAuth2AuthorizedClient> {
        self.authorized_client.as_ref()
    }

    /// Binds `value` to `name` as an attribute of the request.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attribute.
    /// * `value` - The value of the attribute.
    pub fn set_attribute(&mut self, name: impl Into<String>, value: AnyValue) -> &mut Self {
        self.attributes.insert(name.into(), value);
        self
    }
}

/// The state an [`OAuth2AuthorizedClientProvider`] authorizes from.
#[derive(Clone)]
pub struct OAuth2AuthorizationContext {
    client_registration: Option<ClientRegistration>,
    authorized_client: Option<OAuth2AuthorizedClient>,
    principal: Arc<dyn Authentication>,
    attributes: HashMap<String, AnyValue>,
}

impl OAuth2AuthorizationContext {
    /// Creates a context that authorizes `client_registration`.
    ///
    /// # Arguments
    ///
    /// * `client_registration` - The registration of the client.
    /// * `principal` - The principal the authorized client belongs to.
    pub fn with_client_registration(
        client_registration: ClientRegistration,
        principal: Arc<dyn Authentication>,
    ) -> Self {
        Self {
            client_registration: Some(client_registration),
            authorized_client: None,
            principal,
            attributes: HashMap::new(),
        }
    }

    /// Creates a context that re-authorizes `authorized_client`.
    ///
    /// # Arguments
    ///
    /// * `authorized_client` - The client to re-authorize.
    /// * `principal` - The principal the authorized client belongs to.
    pub fn with_authorized_client(
        authorized_client: OAuth2AuthorizedClient,
        principal: Arc<dyn Authentication>,
    ) -> Self {
        Self {
            client_registration: Some(authorized_client.client_registration().clone()),
            authorized_client: Some(authorized_client),
            principal,
            attributes: HashMap::new(),
        }
    }

    /// Returns the registration of the client of the context.
    pub fn client_registration(&self) -> Option<&ClientRegistration> {
        self.client_registration.as_ref()
    }

    /// Returns the authorized client of the context, when it re-authorizes one.
    pub fn authorized_client(&self) -> Option<&OAuth2AuthorizedClient> {
        self.authorized_client.as_ref()
    }

    /// Returns the principal the authorized client belongs to.
    pub fn principal(&self) -> &Arc<dyn Authentication> {
        &self.principal
    }

    /// Returns the attributes of the context.
    pub fn attributes(&self) -> &HashMap<String, AnyValue> {
        &self.attributes
    }

    /// Returns the attribute bound to `name`, if any.
    pub fn attribute(&self, name: &str) -> Option<&AnyValue> {
        self.attributes.get(name)
    }

    /// Binds `value` to `name` as an attribute of the context.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attribute.
    /// * `value` - The value of the attribute.
    pub fn set_attribute(&mut self, name: impl Into<String>, value: AnyValue) -> &mut Self {
        self.attributes.insert(name.into(), value);
        self
    }
}

/// Authorizes (or re-authorizes) an OAuth 2.0 client.
pub trait OAuth2AuthorizedClientProvider: Send + Sync {
    /// Authorizes the client of the context.
    ///
    /// Returns the authorized client, or `None` when the provider does not
    /// support the grant the context asks for, which lets the next provider of
    /// a [`DelegatingOAuth2AuthorizedClientProvider`] run.
    ///
    /// # Errors
    ///
    /// Returns [`OAuth2AuthorizeError::AuthorizationRequired`] when the client
    /// has to obtain authorization from the resource owner first, and
    /// [`OAuth2AuthorizeError::Authentication`] when the attempt failed.
    ///
    /// # Arguments
    ///
    /// * `context` - The state the client is authorized from.
    fn authorize(
        &self,
        context: &mut OAuth2AuthorizationContext,
    ) -> Result<Option<OAuth2AuthorizedClient>, OAuth2AuthorizeError>;
}

/// Delegates to the providers it holds, in order, and returns the first result.
#[derive(Clone, Default)]
pub struct DelegatingOAuth2AuthorizedClientProvider {
    providers: Vec<Arc<dyn OAuth2AuthorizedClientProvider>>,
}

impl DelegatingOAuth2AuthorizedClientProvider {
    /// Creates a provider that delegates to `providers`.
    ///
    /// # Arguments
    ///
    /// * `providers` - The providers to delegate to, in order.
    pub fn new(providers: Vec<Arc<dyn OAuth2AuthorizedClientProvider>>) -> Self {
        Self { providers }
    }

    /// Adds the provider that is delegated to last.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to add.
    pub fn add_provider(&mut self, provider: Arc<dyn OAuth2AuthorizedClientProvider>) -> &mut Self {
        self.providers.push(provider);
        self
    }

    /// Returns the providers in the order they are delegated to.
    pub fn providers(&self) -> &[Arc<dyn OAuth2AuthorizedClientProvider>] {
        &self.providers
    }
}

impl OAuth2AuthorizedClientProvider for DelegatingOAuth2AuthorizedClientProvider {
    fn authorize(
        &self,
        context: &mut OAuth2AuthorizationContext,
    ) -> Result<Option<OAuth2AuthorizedClient>, OAuth2AuthorizeError> {
        for provider in &self.providers {
            if let Some(authorized_client) = provider.authorize(context)? {
                return Ok(Some(authorized_client));
            }
        }

        Ok(None)
    }
}

/// Authorizes a client whose grant type is the authorization code.
///
/// The provider only reports that the client has to obtain authorization from
/// the resource owner: the filter that starts the authorization request
/// recovers the registration from the error and starts the request, which is
/// what the login of the client is completed by.
#[derive(Clone, Default)]
pub struct AuthorizationCodeOAuth2AuthorizedClientProvider;

impl OAuth2AuthorizedClientProvider for AuthorizationCodeOAuth2AuthorizedClientProvider {
    fn authorize(
        &self,
        context: &mut OAuth2AuthorizationContext,
    ) -> Result<Option<OAuth2AuthorizedClient>, OAuth2AuthorizeError> {
        let Some(client_registration) = context.client_registration() else {
            return Ok(None);
        };

        if client_registration.authorization_grant_type()
            == &AuthorizationGrantType::AuthorizationCode
            && context.authorized_client().is_none()
        {
            return Err(ClientAuthorizationRequiredError::new(
                client_registration.registration_id(),
            )
            .into());
        }

        Ok(None)
    }
}

/// Authorizes (or re-authorizes) an OAuth 2.0 client.
pub trait OAuth2AuthorizedClientManager: Send + Sync {
    /// Authorizes the client the request names for its principal.
    ///
    /// The authorized client is loaded from, and saved back to, the authorized
    /// client repository of the manager, so the request and the response are
    /// needed to reach it.
    ///
    /// # Errors
    ///
    /// Returns the error of the provider, which is
    /// [`OAuth2AuthorizeError::AuthorizationRequired`] when the client has to
    /// obtain authorization from the resource owner first.
    ///
    /// # Arguments
    ///
    /// * `authorize_request` - The client to authorize and its principal.
    /// * `request` - The request the authorized client is loaded from.
    /// * `response` - The response the authorized client is saved to.
    fn authorize(
        &self,
        authorize_request: OAuth2AuthorizeRequest,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<Option<OAuth2AuthorizedClient>, OAuth2AuthorizeError>;
}

/// The default [`OAuth2AuthorizedClientManager`] of a request.
///
/// The manager loads the authorized client of the request from the authorized
/// client repository and saves an authorized client back to it, so it is used
/// within the context of a request.
pub struct DefaultOAuth2AuthorizedClientManager {
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    authorized_client_provider: Arc<dyn OAuth2AuthorizedClientProvider>,
}

impl DefaultOAuth2AuthorizedClientManager {
    /// Creates a manager that uses the default providers.
    ///
    /// # Arguments
    ///
    /// * `client_registration_repository` - The repository of the client
    ///   registrations of the application.
    /// * `authorized_client_repository` - The repository of the authorized
    ///   clients of the request.
    pub fn new(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    ) -> Self {
        Self {
            client_registration_repository,
            authorized_client_repository,
            authorized_client_provider: Arc::new(DelegatingOAuth2AuthorizedClientProvider::new(
                vec![Arc::new(AuthorizationCodeOAuth2AuthorizedClientProvider)],
            )),
        }
    }

    /// Sets the provider that authorizes (or re-authorizes) the client.
    ///
    /// # Arguments
    ///
    /// * `authorized_client_provider` - The provider to use.
    pub fn set_authorized_client_provider(
        &mut self,
        authorized_client_provider: Arc<dyn OAuth2AuthorizedClientProvider>,
    ) -> &mut Self {
        self.authorized_client_provider = authorized_client_provider;
        self
    }
}

impl OAuth2AuthorizedClientManager for DefaultOAuth2AuthorizedClientManager {
    fn authorize(
        &self,
        authorize_request: OAuth2AuthorizeRequest,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<Option<OAuth2AuthorizedClient>, OAuth2AuthorizeError> {
        let client_registration_id = authorize_request.client_registration_id();
        let principal = Arc::clone(authorize_request.principal());

        let authorized_client = match authorize_request.authorized_client() {
            Some(authorized_client) => Some(authorized_client.clone()),
            None => self.authorized_client_repository.load_authorized_client(
                client_registration_id,
                principal.as_ref(),
                request,
            ),
        };

        let mut authorization_context = match authorized_client {
            Some(authorized_client) => OAuth2AuthorizationContext::with_authorized_client(
                authorized_client,
                principal.clone(),
            ),
            None => {
                let Some(client_registration) = self
                    .client_registration_repository
                    .find_by_registration_id(client_registration_id)
                else {
                    return Err(AuthenticationError::with_kind(
                        format!(
                            "Could not find ClientRegistration with id '{client_registration_id}'"
                        ),
                        AuthenticationErrorKind::InvalidClientRegistrationId,
                    )
                    .into());
                };

                OAuth2AuthorizationContext::with_client_registration(
                    client_registration,
                    principal.clone(),
                )
            }
        };

        for (name, value) in authorize_request.attributes() {
            authorization_context.set_attribute(name.clone(), value.clone());
        }

        // The scopes a request asks for are read from its `scope` parameter,
        // which is the parameter the attribute is mapped from.
        if authorization_context
            .attribute(REQUEST_SCOPE_ATTRIBUTE_NAME)
            .is_none()
        {
            if let Some(scope) = request.parameter("scope") {
                authorization_context.set_attribute(
                    REQUEST_SCOPE_ATTRIBUTE_NAME,
                    AnyValue::String(scope.to_string()),
                );
            }
        }

        match self
            .authorized_client_provider
            .authorize(&mut authorization_context)
        {
            Ok(Some(authorized_client)) => {
                self.authorized_client_repository.save_authorized_client(
                    authorized_client.clone(),
                    principal.as_ref(),
                    request,
                    response,
                );

                Ok(Some(authorized_client))
            }
            // A provider that does not support re-authorization reports no
            // client, and the client the context carries is kept.
            Ok(None) => Ok(authorization_context.authorized_client().cloned()),
            Err(error) => {
                if matches!(error, OAuth2AuthorizeError::Authentication(_)) {
                    self.authorized_client_repository.remove_authorized_client(
                        client_registration_id,
                        principal.as_ref(),
                        request,
                        response,
                    );
                }

                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use axum::{body::Body, extract::Request, response::Response};

    use super::{
        AuthorizationCodeOAuth2AuthorizedClientProvider, DefaultOAuth2AuthorizedClientManager,
        DelegatingOAuth2AuthorizedClientProvider, OAuth2AuthorizationContext, OAuth2AuthorizeError,
        OAuth2AuthorizeRequest, OAuth2AuthorizedClientManager, OAuth2AuthorizedClientProvider,
    };
    use crate::{
        authentication::UsernamePasswordAuthenticationToken,
        core::Authentication,
        oauth2::{
            ClientRegistration, InMemoryClientRegistrationRepository, OAuth2AccessToken,
            OAuth2AuthorizedClient, OAuth2AuthorizedClientRepository,
        },
    };

    /// A principal that carries only a name.
    fn principal(name: &str) -> std::sync::Arc<dyn Authentication> {
        std::sync::Arc::new(UsernamePasswordAuthenticationToken::authenticated(
            std::sync::Arc::new(name.to_string()),
            None,
            Vec::new(),
        ))
    }

    /// A registration with the default grant type.
    fn registration() -> ClientRegistration {
        ClientRegistration::new("github", "GitHub", "https://example.com/oauth2/authorize")
    }

    /// A repository that keeps the authorized clients in memory.
    #[derive(Default)]
    struct InMemoryAuthorizedClientRepository {
        authorized_clients: Mutex<std::collections::HashMap<String, OAuth2AuthorizedClient>>,
    }

    impl OAuth2AuthorizedClientRepository for InMemoryAuthorizedClientRepository {
        fn save_authorized_client(
            &self,
            authorized_client: OAuth2AuthorizedClient,
            _principal: &dyn Authentication,
            _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
            _response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
        ) {
            self.authorized_clients.lock().unwrap().insert(
                authorized_client
                    .client_registration()
                    .registration_id()
                    .to_string(),
                authorized_client,
            );
        }

        fn load_authorized_client(
            &self,
            client_registration_id: &str,
            _principal: &dyn Authentication,
            _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        ) -> Option<OAuth2AuthorizedClient> {
            self.authorized_clients
                .lock()
                .unwrap()
                .get(client_registration_id)
                .cloned()
        }
    }

    /// A provider that reports the client it holds.
    struct FixedProvider(Option<OAuth2AuthorizedClient>);

    impl OAuth2AuthorizedClientProvider for FixedProvider {
        fn authorize(
            &self,
            _context: &mut OAuth2AuthorizationContext,
        ) -> Result<Option<OAuth2AuthorizedClient>, OAuth2AuthorizeError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn the_authorization_code_provider_requires_authorization_of_an_unauthorized_client() {
        let mut context =
            OAuth2AuthorizationContext::with_client_registration(registration(), principal("user"));

        let error = AuthorizationCodeOAuth2AuthorizedClientProvider
            .authorize(&mut context)
            .expect_err("the client has to obtain authorization first");

        match error {
            OAuth2AuthorizeError::AuthorizationRequired(error) => {
                assert_eq!(error.client_registration_id(), "github")
            }
            OAuth2AuthorizeError::Authentication(error) => {
                panic!("the provider has to ask for authorization, but failed: {error}")
            }
        }
    }

    #[test]
    fn the_delegating_provider_returns_the_first_client_reported() {
        let client =
            OAuth2AuthorizedClient::new(registration(), "user", OAuth2AccessToken::new("first"));
        let later =
            OAuth2AuthorizedClient::new(registration(), "user", OAuth2AccessToken::new("later"));
        let provider = DelegatingOAuth2AuthorizedClientProvider::new(vec![
            std::sync::Arc::new(FixedProvider(None)),
            std::sync::Arc::new(FixedProvider(Some(client))),
            std::sync::Arc::new(FixedProvider(Some(later))),
        ]);

        let mut context =
            OAuth2AuthorizationContext::with_client_registration(registration(), principal("user"));
        let authorized_client = provider
            .authorize(&mut context)
            .expect("none of the providers fails")
            .expect("a provider reports a client");

        assert_eq!(authorized_client.access_token().token_value(), "first");
    }

    #[test]
    fn the_default_manager_returns_the_authorized_client_of_the_repository() {
        let authorized_client =
            OAuth2AuthorizedClient::new(registration(), "user", OAuth2AccessToken::new("token"));
        let authorized_client_repository =
            std::sync::Arc::new(InMemoryAuthorizedClientRepository::default());
        let mut request = Request::builder().body(Body::empty()).unwrap();
        let mut response = Response::new(Body::empty());

        authorized_client_repository.save_authorized_client(
            authorized_client,
            principal("user").as_ref(),
            &request,
            &mut response,
        );

        let authorized_client_manager = DefaultOAuth2AuthorizedClientManager::new(
            std::sync::Arc::new(InMemoryClientRegistrationRepository::new([registration()])),
            authorized_client_repository,
        );

        let authorized_client = authorized_client_manager
            .authorize(
                OAuth2AuthorizeRequest::with_client_registration("github", principal("user")),
                &request,
                &mut response,
            )
            .expect("the authorization succeeds")
            .expect("the repository holds an authorized client");

        assert_eq!(authorized_client.access_token().token_value(), "token");
    }
}

/// A client that has been authorized to access a protected resource on behalf
/// of a principal, together with the issued access (and optional refresh) token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AuthorizedClient {
    client_registration: ClientRegistration,
    principal_name: String,
    access_token: OAuth2AccessToken,
    refresh_token: Option<OAuth2RefreshToken>,
}

impl OAuth2AuthorizedClient {
    pub fn new(
        client_registration: ClientRegistration,
        principal_name: impl Into<String>,
        access_token: OAuth2AccessToken,
    ) -> Self {
        Self {
            client_registration,
            principal_name: principal_name.into(),
            access_token,
            refresh_token: None,
        }
    }

    pub fn with_refresh_token(mut self, refresh_token: OAuth2RefreshToken) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    pub fn principal_name(&self) -> &str {
        &self.principal_name
    }

    pub fn access_token(&self) -> &OAuth2AccessToken {
        &self.access_token
    }

    pub fn refresh_token(&self) -> Option<&OAuth2RefreshToken> {
        self.refresh_token.as_ref()
    }
}

impl Identity for OAuth2AuthorizedClient {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Display for OAuth2AuthorizedClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.principal_name)
    }
}
