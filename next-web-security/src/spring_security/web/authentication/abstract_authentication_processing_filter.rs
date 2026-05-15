use std::sync::Arc;

use axum::{extract::Request, http::StatusCode, response::Response};
use next_web_core::{
    anys::{any_map::AnyMap, any_value::AnyValue},
    error::BoxError,
};

use crate::{
    authorization::authentication_manager::AuthenticationManager,
    config::web::util::matcher::request_matcher::RequestMatcher,
    core::{
        authentication::Authentication,
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
        context::{security_context::SecurityContext, security_context_holder::SecurityContextHolder},
    },
    web::authentication::{
        authentication_failure_handler::AuthenticationFailureHandler,
        authentication_success_handler::AuthenticationSuccessHandler,
        preauth::abstract_pre_authenticated_processing_filter::{
            block_on, NEXT_SECURITY_AUTHENTICATION,
        },
        remember_me_services::RememberMeServices,
        rememberme::abstract_remember_me_services::AbstractRememberMeServices,
    },
};

#[derive(Clone)]
pub struct AbstractAuthenticationProcessingFilter {
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    success_handler: Option<Arc<dyn AuthenticationSuccessHandler>>,
    failure_handler: Option<Arc<dyn AuthenticationFailureHandler>>,
    remember_me_services: Arc<dyn RememberMeServices>,
    requires_authentication_request_matcher: Option<Box<dyn RequestMatcher>>,
}

impl Default for AbstractAuthenticationProcessingFilter {
    fn default() -> Self {
        Self {
            authentication_manager: None,
            success_handler: None,
            failure_handler: None,
            remember_me_services: Arc::new(AbstractRememberMeServices {}),
            requires_authentication_request_matcher: None,
        }
    }
}

impl AbstractAuthenticationProcessingFilter {
    pub fn get_remember_me_services(&self) -> &dyn RememberMeServices {
        self.remember_me_services.as_ref()
    }

    pub fn set_authentication_manager(&mut self, authentication_manager: Arc<dyn AuthenticationManager>) {
        self.authentication_manager = Some(authentication_manager);
    }

    pub fn set_success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = Some(success_handler);
    }

    pub fn set_failure_handler(&mut self, failure_handler: Arc<dyn AuthenticationFailureHandler>) {
        self.failure_handler = Some(failure_handler);
    }

    pub fn requires_authentication(&self, request: &Request) -> bool {
        self.requires_authentication_request_matcher
            .as_ref()
            .map(|matcher| matcher.matches(request))
            .unwrap_or(false)
    }

    pub fn attempt_authentication(
        &self,
        request: &Request,
        response: &mut Response,
        authentication: &dyn Authentication,
    ) -> Result<Option<Arc<dyn Authentication>>, BoxError> {
        let Some(authentication_manager) = &self.authentication_manager else {
            let error = AuthenticationError::with_kind(
                "AuthenticationManager must be specified",
                AuthenticationErrorKind::ProviderNotFound,
            );
            self.unsuccessful_authentication(request, response, error.clone());
            return Err(Box::new(error));
        };

        match authentication_manager.authenticate(authentication) {
            Ok(authentication) => {
                self.successful_authentication(request, response, authentication.clone());
                Ok(Some(authentication))
            }
            Err(error) => {
                self.unsuccessful_authentication(request, response, error);
                Ok(None)
            }
        }
    }

    fn successful_authentication(
        &self,
        request: &Request,
        response: &mut Response,
        authentication: Arc<dyn Authentication>,
    ) {
        let mut context = SecurityContext::new(None);
        context.set_authentication(Some(authentication.clone()));
        SecurityContextHolder::set_context(context);

        if let Some(any_map) = request.extensions().get::<AnyMap>() {
            block_on(any_map.insert(
                NEXT_SECURITY_AUTHENTICATION.to_string(),
                AnyValue::Object(Box::new(authentication.clone())),
            ));
        }

        if let Some(success_handler) = &self.success_handler {
            success_handler.on_authentication_success(request, response, authentication.as_ref());
        }
    }

    fn unsuccessful_authentication(
        &self,
        request: &Request,
        response: &mut Response,
        error: AuthenticationError,
    ) {
        SecurityContextHolder::clear_context();
        if let Some(any_map) = request.extensions().get::<AnyMap>() {
            block_on(any_map.insert(
                "NEXT_SECURITY_LAST_ERROR".to_string(),
                AnyValue::Object(Box::new(error.clone())),
            ));
        }

        if let Some(failure_handler) = &self.failure_handler {
            failure_handler.on_authentication_failure(request, response, &error);
        } else {
            *response.status_mut() = StatusCode::UNAUTHORIZED;
        }
    }
}

impl AbstractAuthenticationProcessingFilter {
    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: impl RequestMatcher + 'static,
    ) {
        self.requires_authentication_request_matcher = Some(Box::new(request_matcher));
    }
}
