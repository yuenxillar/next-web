use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

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
    core::{AuthenticationError, AuthenticationErrorKind},
    web::authentication::{
        preauth::{
            base_pre_authenticated_processing_filter::BasePreAuthenticatedProcessingFilter,
            BasePreAuthenticatedProcessingFilterExt,
        },
        AuthPrincipal,
    },
};

/// A simple pre-authenticated filter which obtains the username from a request header,
/// for use with systems such as CA Siteminder.
///
/// As with most pre-authenticated scenarios, it is essential that the external
/// authentication system is set up correctly as this filter does no authentication
/// whatsoever. All the protection is assumed to be provided externally and if this
/// filter is included inappropriately in a configuration, it would be possible to
/// assume the identity of a user merely by setting the correct header name. This also
/// means it should not generally be used in combination with other Spring Security
/// authentication mechanisms such as form login, as this would imply there was a means
/// of bypassing the external system which would be risky.
///
/// The property [`principal_request_header`](Self::principal_request_header) is the name
/// of the request header that contains the username. It defaults to `"SM_USER"` for
/// compatibility with Siteminder.
///
/// If the header is missing from the request,
/// [`get_pre_authenticated_principal`](Self::get_pre_authenticated_principal) will throw
/// an error. You can override this behaviour by setting the
/// [`error_if_header_missing`](Self::error_if_header_missing) property.
#[derive(Clone)]
pub struct RequestHeaderAuthenticationFilter {
    principal_request_header: String,
    credentials_request_header: Option<String>,
    error_if_header_missing: bool,
    base: BasePreAuthenticatedProcessingFilter,
}

impl RequestHeaderAuthenticationFilter {
    pub fn set_principal_request_header(&mut self, principal_request_header: impl Into<String>) {
        let principal_request_header = principal_request_header.into();
        assert!(
            !principal_request_header.trim().is_empty(),
            "principal_request_header must not be empty"
        );
        self.principal_request_header = principal_request_header;
    }

    pub fn set_credentials_request_header(
        &mut self,
        credentials_request_header: impl Into<String>,
    ) {
        let credentials_request_header = credentials_request_header.into();
        assert!(
            !credentials_request_header.trim().is_empty(),
            "credentials_request_header must not be empty"
        );
        self.credentials_request_header = Some(credentials_request_header);
    }

    /// Defines whether an error should be raised if the principal header is missing. Defaults to true.
    pub fn set_error_if_header_missing(&mut self, error_if_header_missing: bool) {
        self.error_if_header_missing = error_if_header_missing;
    }
}

impl BasePreAuthenticatedProcessingFilterExt for RequestHeaderAuthenticationFilter {
    /// Read and returns the header named by principalRequestHeader from the request.
    fn get_pre_authenticated_principal(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError> {
        match request
            .header(&self.principal_request_header)
            .map(ToString::to_string)
        {
            Some(principal) => Ok(Some(Arc::new(principal))),
            None => {
                if self.error_if_header_missing {
                    Err(AuthenticationError::with_kind(
                        format!(
                            "{} header not found in request.",
                            self.principal_request_header
                        ),
                        AuthenticationErrorKind::CredentialsNotFound,
                    ))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Credentials aren't usually applicable, but if a credentialsRequestHeader is set,
    /// this will be read and used as the credentials value. Otherwise a dummy value will be used.
    fn get_pre_authenticated_credentials(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError> {
        if let Some(name) = self.credentials_request_header.as_deref() {
            return request
                .header(name)
                .map(|value| {
                    Ok::<_, AuthenticationError>(Arc::new(value.to_string()) as AuthPrincipal)
                })
                .transpose();
        }

        Ok(Some(Arc::new(String::from("N/A"))))
    }
}

#[async_trait]
impl HttpFilter for RequestHeaderAuthenticationFilter {
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

impl Named for RequestHeaderAuthenticationFilter {
    fn name(&self) -> &str {
        "RequestHeaderAuthenticationFilter"
    }
}

impl Deref for RequestHeaderAuthenticationFilter {
    type Target = BasePreAuthenticatedProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for RequestHeaderAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
