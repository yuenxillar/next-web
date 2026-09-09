use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationManager,
    core::{context::SecurityContextHolder, Authentication, AuthenticationError},
};

pub const NEXT_SECURITY_AUTHENTICATION: &str = "NEXT_SECURITY_AUTHENTICATION";
pub const NEXT_SECURITY_REQUEST_ATTRIBUTES: &str = "NEXT_SECURITY_REQUEST_ATTRIBUTES";

#[derive(Clone)]
pub struct BasePreAuthenticatedProcessingFilter {
    authentication_manager: Arc<dyn AuthenticationManager>,
    continue_filter_chain_on_unsuccessful_authentication: bool,
}

impl BasePreAuthenticatedProcessingFilter {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            authentication_manager,
            continue_filter_chain_on_unsuccessful_authentication: true,
        }
    }

    pub fn set_continue_filter_chain_on_unsuccessful_authentication(&mut self, value: bool) {
        self.continue_filter_chain_on_unsuccessful_authentication = value;
    }

    pub async fn authenticate(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    ) -> Result<Option<Arc<dyn Authentication>>, BoxError> {
        match self
            .authentication_manager
            .authenticate(authentication)
            .await
        {
            Ok(authentication) => {
                self.successful_authentication(request, response, authentication.clone())?;
                Ok(Some(authentication))
            }
            Err(error) => {
                self.unsuccessful_authentication(request, error.clone())?;
                if self.continue_filter_chain_on_unsuccessful_authentication {
                    Ok(None)
                } else {
                    Err(Box::new(error))
                }
            }
        }
    }

    fn successful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        authentication: Arc<dyn Authentication>,
    ) -> Result<(), FilterError> {
        let context = SecurityContextHolder::create_empty_context();
        context.set_authentication(Some(authentication.clone()));
        SecurityContextHolder::set_context(context);
        request.set_attribute(
            NEXT_SECURITY_AUTHENTICATION,
            AnyValue::Object(Box::new(authentication)),
        );
        Ok(())
    }

    fn unsuccessful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        error: AuthenticationError,
    ) -> Result<(), BoxError> {
        SecurityContextHolder::clear_context();
        request.set_attribute(
            "NEXT_SECURITY_LAST_ERROR",
            AnyValue::Object(Box::new(error)),
        );
        Ok(())
    }
}

#[async_trait]
impl HttpFilter for BasePreAuthenticatedProcessingFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        filter_chain.do_filter(request, response).await
    }
}

impl Named for BasePreAuthenticatedProcessingFilter {
    fn name(&self) -> &str {
        "BasePreAuthenticatedProcessingFilter"
    }
}
