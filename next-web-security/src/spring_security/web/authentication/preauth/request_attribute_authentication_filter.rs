use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationManager,
    core::{AuthenticationError, AuthenticationErrorKind},
    web::authentication::{
        preauth::base_pre_authenticated_processing_filter::{
            BasePreAuthenticatedProcessingFilter, BasePreAuthenticatedProcessingFilterExt,
        },
        AuthPrincipal,
    },
};

/// A simple pre-authenticated filter which obtains the username from request attributes, for use with
///  SSO systems such as Stanford WebAuth   or Shibboleth  .
///
/// As with most pre-authenticated scenarios, it is essential that the external authentication system
/// is set up correctly as this filter does no authentication whatsoever.
///
/// The property principalEnvironmentVariable is the name of the request attribute that contains the username.
/// It defaults to "REMOTE_USER" for compatibility with WebAuth and Shibboleth.
///
/// If the environment variable is missing from the request, getPreAuthenticatedPrincipal will throw an exception.
/// You can override this behaviour by setting the exceptionIfVariableMissing property
#[derive(Clone)]
pub struct RequestAttributeAuthenticationFilter {
    principal_environment_variable: String,
    credentials_environment_variable: Option<String>,
    exception_if_variable_missing: bool,

    base: BasePreAuthenticatedProcessingFilter,
}

impl RequestAttributeAuthenticationFilter {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            principal_environment_variable: String::from("REMOTE_USER"),
            credentials_environment_variable: None,
            exception_if_variable_missing: true,
            base: BasePreAuthenticatedProcessingFilter::new(authentication_manager),
        }
    }

    pub fn set_principal_environment_variable(
        &mut self,
        principal_environment_variable: impl Into<String>,
    ) {
        let principal_environment_variable = principal_environment_variable.into();
        assert!(
            !principal_environment_variable.trim().is_empty(),
            "principalEnvironmentVariable must not be empty or null"
        );
        self.principal_environment_variable = principal_environment_variable;
    }

    pub fn set_credentials_environment_variable(
        &mut self,
        credentials_environment_variable: impl Into<String>,
    ) {
        let credentials_environment_variable = credentials_environment_variable.into();
        assert!(
            !credentials_environment_variable.trim().is_empty(),
            "credentialsEnvironmentVariable must not be empty or null"
        );
        self.credentials_environment_variable = Some(credentials_environment_variable);
    }

    pub fn set_exception_if_variable_missing(&mut self, exception_if_variable_missing: bool) {
        self.exception_if_variable_missing = exception_if_variable_missing;
    }
}

impl BasePreAuthenticatedProcessingFilterExt for RequestAttributeAuthenticationFilter {
    /// Reads the request attribute named by `principal_environment_variable` as the
    /// pre-authenticated principal.
    fn get_pre_authenticated_principal(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError> {
        let principal = request_attribute(request, &self.principal_environment_variable);
        if principal.is_none() && self.exception_if_variable_missing {
            return Err(AuthenticationError::with_kind(
                format!(
                    "{} variable not found in request.",
                    self.principal_environment_variable
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }
        Ok(principal.map(|principal| Arc::new(principal) as AuthPrincipal))
    }

    /// Credentials aren't usually applicable, but if a credentials environment variable
    /// is set, this will be read and used as the credentials value. Otherwise a dummy
    /// value will be used.
    fn get_pre_authenticated_credentials(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError> {
        let credentials = self
            .credentials_environment_variable
            .as_ref()
            .and_then(|name| request_attribute(request, name))
            .unwrap_or_else(|| String::from("N/A"));
        Ok(Some(Arc::new(credentials) as AuthPrincipal))
    }
}

#[async_trait]
impl HttpFilter for RequestAttributeAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        self.base
            .do_filter(self, request, response, filter_chain)
            .await
    }
}

impl Named for RequestAttributeAuthenticationFilter {
    fn name(&self) -> &str {
        "RequestAttributeAuthenticationFilter"
    }
}

fn request_attribute(request: &dyn HttpRequest, key: &str) -> Option<String> {
    request
        .get_attribute(key)
        .and_then(|value| value.as_string())
}
