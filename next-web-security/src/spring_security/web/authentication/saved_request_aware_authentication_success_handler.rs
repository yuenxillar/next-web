use std::{ops::{Deref, DerefMut}, sync::Arc};

use axum::extract::Request;

use crate::{
    core::authentication::Authentication,
    web::{
        authentication::{
            abstract_authentication_target_url_request_handler::AbstractAuthenticationTargetUrlRequestHandler,
            authentication_success_handler::AuthenticationSuccessHandler,
        },
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
        savedrequest::{
            http_session_request_cache::HttpSessionRequestCache, request_cache::RequestCache,
        },
    },
};

pub struct SavedRequestAwareAuthenticationSuccessHandler {
    request_cache: Arc<dyn RequestCache>,

    abstract_authentication_target_url_request_handler:
        AbstractAuthenticationTargetUrlRequestHandler,
}

impl Deref for SavedRequestAwareAuthenticationSuccessHandler {
    type Target = AbstractAuthenticationTargetUrlRequestHandler;

    fn deref(&self) -> &Self::Target {
        &self.abstract_authentication_target_url_request_handler
    }
}

impl DerefMut for SavedRequestAwareAuthenticationSuccessHandler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.abstract_authentication_target_url_request_handler
    }
}

impl SavedRequestAwareAuthenticationSuccessHandler {
    pub fn new() -> Self {
        Self {
            request_cache: Arc::new(HttpSessionRequestCache::new()),
            abstract_authentication_target_url_request_handler:
                AbstractAuthenticationTargetUrlRequestHandler::default(),
        }
    }

    pub fn on_authentication_success(
        &self,
        request: &Request,
        _authentication: &dyn Authentication,
    ) -> Option<String> {
        let save_request = self.request_cache.get_request(request);
        if let Some(save_request) = save_request {
            let target_url_parameter = self.get_target_url_parameter();
            if target_url_parameter.is_empty() && !self.is_always_use_default_target_url() {
                return Some(save_request.get_redirect_url());
            }
        }

        None
    }

    pub fn set_request_cache(&mut self, request_cache: Arc<dyn RequestCache>) {
        self.request_cache = request_cache;
    }

    pub fn set_default_target_url(&mut self, default_success_url: impl ToString) {
        self.abstract_authentication_target_url_request_handler
            .set_default_target_url(default_success_url.to_string());
    }

    pub fn set_always_use_default_target_url(&mut self, always_use: bool) {
        self.abstract_authentication_target_url_request_handler
            .set_always_use_default_target_url(always_use);
    }
}

impl AuthenticationSuccessHandler for SavedRequestAwareAuthenticationSuccessHandler {
    fn on_authentication_success(
        &self,
        request: &Request,
        response: &mut axum::response::Response,
        authentication: &dyn Authentication,
    ) {
        if let Some(target_url) =
            SavedRequestAwareAuthenticationSuccessHandler::on_authentication_success(
                self,
                request,
                authentication,
            )
        {
            DefaultRedirectStrategy::default().send_redirect(None, &target_url, response);
            return;
        }

        self.abstract_authentication_target_url_request_handler
            .handle(request, response);
    }
}
