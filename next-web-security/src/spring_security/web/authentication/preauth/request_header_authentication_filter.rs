use std::sync::Arc;

use axum::{extract::Request, response::Response};
use next_web_core::error::BoxError;

use crate::{
    authorization::authentication_manager::AuthenticationManager,
    core::filter::Filter,
    web::authentication::preauth::{
        abstract_pre_authenticated_processing_filter::AbstractPreAuthenticatedProcessingFilterSupport,
        pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
        pre_authenticated_credentials_not_found_exception::pre_authenticated_credentials_not_found,
    },
};

#[derive(Clone)]
pub struct RequestHeaderAuthenticationFilter {
    support: AbstractPreAuthenticatedProcessingFilterSupport,
    principal_request_header: String,
    credentials_request_header: Option<String>,
    exception_if_header_missing: bool,
}

impl RequestHeaderAuthenticationFilter {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            support: AbstractPreAuthenticatedProcessingFilterSupport::new(authentication_manager),
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
        request: &Request,
    ) -> Result<Option<String>, crate::core::authentication_error::AuthenticationError> {
        let principal = request
            .headers()
            .get(&self.principal_request_header)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        if principal.is_none() && self.exception_if_header_missing {
            return Err(pre_authenticated_credentials_not_found(format!(
                "{} header not found in request.",
                self.principal_request_header
            )));
        }
        Ok(principal)
    }

    pub fn pre_authenticated_credentials(&self, request: &Request) -> Option<String> {
        self.credentials_request_header
            .as_ref()
            .and_then(|header| request.headers().get(header))
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned)
            .or_else(|| Some(String::from("N/A")))
    }
}

impl Filter for RequestHeaderAuthenticationFilter {
    fn do_filter(&self, req: &mut Request, res: &mut Response) -> Result<(), BoxError> {
        let principal = match self.pre_authenticated_principal(req) {
            Ok(principal) => principal,
            Err(error) => return Err(Box::new(error)),
        };
        let Some(principal) = principal else {
            return Ok(());
        };

        let token = PreAuthenticatedAuthenticationToken::unauthenticated(
            Some(principal),
            self.pre_authenticated_credentials(req),
        );
        let _ = self.support.authenticate(req, res, &token)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, extract::Request, http::Request as HttpRequest};

    use crate::{
        authorization::authentication_manager::AuthenticationManager,
        core::{authentication::Authentication, authority_utils::AuthorityUtils},
    };

    use super::RequestHeaderAuthenticationFilter;

    struct StubAuthenticationManager;

    impl AuthenticationManager for StubAuthenticationManager {
        fn authenticate(
            &self,
            authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, crate::core::authentication_error::AuthenticationError> {
            Ok(Arc::new(
                crate::web::authentication::preauth::pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken::authenticated(
                    authentication.get_name(),
                    authentication.get_credentials(),
                    AuthorityUtils::create_authority_list(["ROLE_USER"]),
                ),
            ))
        }
    }

    #[test]
    fn request_header_filter_reads_principal_and_credentials_headers() {
        let mut filter = RequestHeaderAuthenticationFilter::new(Arc::new(StubAuthenticationManager));
        filter.set_principal_request_header("x-user");
        filter.set_credentials_request_header("x-credential");

        let request = HttpRequest::builder()
            .header("x-user", "alice")
            .header("x-credential", "external")
            .body(Body::empty())
            .unwrap();
        let request = Request::from(request);

        assert_eq!(
            filter.pre_authenticated_principal(&request).unwrap(),
            Some(String::from("alice"))
        );
        assert_eq!(
            filter.pre_authenticated_credentials(&request),
            Some(String::from("external"))
        );
    }
}
