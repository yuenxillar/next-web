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
    web::authentication::preauth::{
        base_pre_authenticated_processing_filter::BasePreAuthenticatedProcessingFilterSupport,
        pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
        pre_authenticated_credentials_not_found_exception::pre_authenticated_credentials_not_found,
    },
};

#[derive(Clone)]
pub struct RequestHeaderAuthenticationFilter {
    support: BasePreAuthenticatedProcessingFilterSupport,
    principal_request_header: String,
    credentials_request_header: Option<String>,
    exception_if_header_missing: bool,
}

impl RequestHeaderAuthenticationFilter {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            support: BasePreAuthenticatedProcessingFilterSupport::new(authentication_manager),
            principal_request_header: String::from("SM_USER"),
            credentials_request_header: None,
            exception_if_header_missing: true,
        }
    }

    pub fn set_principal_request_header(&mut self, principal_request_header: impl Into<String>) {
        let principal_request_header = principal_request_header.into();
        assert!(
            !principal_request_header.trim().is_empty(),
            "principalRequestHeader must not be empty or null"
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
            "credentialsRequestHeader must not be empty or null"
        );
        self.credentials_request_header = Some(credentials_request_header);
    }

    pub fn set_exception_if_header_missing(&mut self, exception_if_header_missing: bool) {
        self.exception_if_header_missing = exception_if_header_missing;
    }

    pub fn pre_authenticated_principal(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<String>, crate::core::authentication_error::AuthenticationError> {
        let principal = request
            .header(&self.principal_request_header)
            .map(ToOwned::to_owned);
        if principal.is_none() && self.exception_if_header_missing {
            return Err(pre_authenticated_credentials_not_found(format!(
                "{} header not found in request.",
                self.principal_request_header
            )));
        }
        Ok(principal)
    }

    pub fn pre_authenticated_credentials(&self, request: &dyn HttpRequest) -> Option<String> {
        self.credentials_request_header
            .as_ref()
            .and_then(|header| request.header(header))
            .map(ToOwned::to_owned)
            .or_else(|| Some(String::from("N/A")))
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
        let principal = match self.pre_authenticated_principal(request) {
            Ok(principal) => principal,
            Err(error) => return Err(FilterError::Chain(Box::new(error))),
        };
        let Some(principal) = principal else {
            return Ok(());
        };

        let token = PreAuthenticatedAuthenticationToken::unauthenticated(
            Some(principal),
            self.pre_authenticated_credentials(request),
        );
        let _ = self.support.authenticate(request, response, &token)?;
        Ok(())
    }
}

impl Named for RequestHeaderAuthenticationFilter {
    fn name(&self) -> &str {
        "RequestHeaderAuthenticationFilter"
    }
}
