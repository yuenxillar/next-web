use std::sync::Arc;

use next_web_core::http::StatusCode;
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
    util::http_method::HttpMethod,
};

use crate::{
    authorization::AuthenticationManager,
    core::{
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
        context::security_context_holder::SecurityContextHolder,
        Authentication,
    },
    web::{
        authentication::{
            authentication_converter::AuthenticationConverter,
            authentication_failure_handler::AuthenticationFailureHandler,
            authentication_success_handler::AuthenticationSuccessHandler,
            remember_me_services::RememberMeServices,
            rememberme::base_remember_me_services::BaseRememberMeServices,
        },
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
    },
};

#[derive(Clone)]
pub struct BaseAuthenticationProcessingFilter {
    authentication_converter: Option<Arc<dyn AuthenticationConverter>>,

    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    success_handler: Option<Arc<dyn AuthenticationSuccessHandler>>,
    failure_handler: Option<Arc<dyn AuthenticationFailureHandler>>,
    remember_me_services: Arc<dyn RememberMeServices>,
    requires_authentication_request_matcher: Option<Arc<dyn RequestMatcher>>,
}

impl Default for BaseAuthenticationProcessingFilter {
    fn default() -> Self {
        Self {
            authentication_converter: Default::default(),
            authentication_manager: Default::default(),
            success_handler: Default::default(),
            failure_handler: Default::default(),
            remember_me_services: Arc::new(BaseRememberMeServices {}),
            requires_authentication_request_matcher: None,
        }
    }
}

impl BaseAuthenticationProcessingFilter {
    pub fn new(default_filter_processes_url: impl Into<String>) -> Self {
        Self {
            authentication_converter: Default::default(),
            authentication_manager: Default::default(),
            success_handler: Default::default(),
            failure_handler: Default::default(),
            remember_me_services: Arc::new(BaseRememberMeServices {}),
            requires_authentication_request_matcher: Some(Arc::new(
                PathPatternRequestMatcher::path_pattern(
                    Some(HttpMethod::Post),
                    default_filter_processes_url.into().as_str(),
                ),
            )),
        }
    }
    pub fn get_remember_me_services(&self) -> &dyn RememberMeServices {
        self.remember_me_services.as_ref()
    }

    pub fn set_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) {
        self.authentication_manager = Some(authentication_manager);
    }

    pub fn set_success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = Some(success_handler);
    }

    pub fn set_failure_handler(&mut self, failure_handler: Arc<dyn AuthenticationFailureHandler>) {
        self.failure_handler = Some(failure_handler);
    }

    pub fn requires_authentication(&self, request: &dyn HttpRequest) -> bool {
        self.requires_authentication_request_matcher
            .as_ref()
            .map(|matcher| matcher.matches(request))
            .unwrap_or(false)
    }

    pub fn set_authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) {
        self.authentication_converter = Some(authentication_converter);
    }

    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: impl RequestMatcher + 'static,
    ) {
        self.requires_authentication_request_matcher = Some(Arc::new(request_matcher));
    }

    pub fn attempt_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
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
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Arc<dyn Authentication>,
    ) {
        // let mut context = SecurityContext::new(None);
        // context.set_authentication(Some(authentication.clone()));
        // SecurityContextHolder::set_context(context);

        // request.set_attribute(
        //     NEXT_SECURITY_AUTHENTICATION,
        //     AnyValue::Object(Box::new(authentication.clone())),
        // );

        // if let Some(success_handler) = &self.success_handler {
        //     success_handler.on_authentication_success(request, response, authentication.as_ref());
        // }
        //
        todo!()
    }

    fn unsuccessful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: AuthenticationError,
    ) {
        SecurityContextHolder::clear_context();

        request.set_attribute(
            "NEXT_SECURITY_LAST_ERROR",
            AnyValue::Object(Box::new(error.clone())),
        );

        if let Some(failure_handler) = &self.failure_handler {
            failure_handler.on_authentication_failure(request, response, &error);
        } else {
            response.set_status_code(StatusCode::UNAUTHORIZED);
        }
    }
}

#[async_trait]
impl HttpFilter for BaseAuthenticationProcessingFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        todo!()
    }
}

impl Named for BaseAuthenticationProcessingFilter {
    fn name(&self) -> &str {
        "BaseAuthenticationProcessingFilter"
    }
}
