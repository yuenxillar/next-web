use std::sync::Arc;

use axum::{extract::Request, response::Response};
use next_web_core::{anys::any_map::AnyMap, error::BoxError};

use crate::{
    authorization::authentication_manager::AuthenticationManager,
    core::filter::Filter,
    web::authentication::preauth::{
        abstract_pre_authenticated_processing_filter::{
            AbstractPreAuthenticatedProcessingFilterSupport, NEXT_SECURITY_REQUEST_ATTRIBUTES,
            block_on,
        },
        pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
        pre_authenticated_credentials_not_found_exception::pre_authenticated_credentials_not_found,
    },
};

#[derive(Clone)]
pub struct RequestAttributeAuthenticationFilter {
    support: AbstractPreAuthenticatedProcessingFilterSupport,
    principal_environment_variable: String,
    credentials_environment_variable: Option<String>,
    exception_if_variable_missing: bool,
}

impl RequestAttributeAuthenticationFilter {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            support: AbstractPreAuthenticatedProcessingFilterSupport::new(authentication_manager),
            principal_environment_variable: String::from("REMOTE_USER"),
            credentials_environment_variable: None,
            exception_if_variable_missing: true,
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

    pub fn pre_authenticated_principal(
        &self,
        request: &Request,
    ) -> Result<Option<String>, crate::core::authentication_error::AuthenticationError> {
        let principal = request_attribute(request, &self.principal_environment_variable);
        if principal.is_none() && self.exception_if_variable_missing {
            return Err(pre_authenticated_credentials_not_found(format!(
                "{} variable not found in request.",
                self.principal_environment_variable
            )));
        }
        Ok(principal)
    }

    pub fn pre_authenticated_credentials(&self, request: &Request) -> Option<String> {
        self.credentials_environment_variable
            .as_ref()
            .and_then(|name| request_attribute(request, name))
            .or_else(|| Some(String::from("N/A")))
    }
}

impl Filter for RequestAttributeAuthenticationFilter {
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

fn request_attribute(request: &Request, key: &str) -> Option<String> {
    let map = request.extensions().get::<AnyMap>()?;
    let value = block_on(map.get(NEXT_SECURITY_REQUEST_ATTRIBUTES))?;
    let attributes = value.as_map()?;
    attributes.get(key).and_then(|value| value.as_string())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{body::Body, extract::Request, http::Request as HttpRequest};
    use next_web_core::anys::{any_map::AnyMap, any_value::AnyValue};

    use crate::{
        authorization::authentication_manager::AuthenticationManager,
        core::{authentication::Authentication, authority_utils::AuthorityUtils},
    };

    use super::{block_on, RequestAttributeAuthenticationFilter, NEXT_SECURITY_REQUEST_ATTRIBUTES};

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
    fn request_attribute_filter_reads_principal_and_credentials_attributes() {
        let mut filter = RequestAttributeAuthenticationFilter::new(Arc::new(StubAuthenticationManager));
        filter.set_principal_environment_variable("principal");
        filter.set_credentials_environment_variable("credential");

        let map = AnyMap::new();
        block_on(map.insert(
            NEXT_SECURITY_REQUEST_ATTRIBUTES.to_string(),
            AnyValue::Map(HashMap::from([
                (String::from("principal"), AnyValue::from("alice")),
                (String::from("credential"), AnyValue::from("external")),
            ])),
        ));

        let mut request = Request::from(HttpRequest::builder().body(Body::empty()).unwrap());
        request.extensions_mut().insert(map);

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
