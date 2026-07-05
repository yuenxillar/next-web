use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{
    core::Authentication,
    web::{
        authentication::{
            authentication_success_handler::AuthenticationSuccessHandler,
            BaseAuthenticationTargetUrlRequestHandler,
        },
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
        savedrequest::{HttpSessionRequestCache, RequestCache},
    },
};

pub struct SavedRequestAwareAuthenticationSuccessHandler {
    request_cache: Arc<dyn RequestCache>,

    base_authentication_target_url_request_handler: BaseAuthenticationTargetUrlRequestHandler,
}

impl Deref for SavedRequestAwareAuthenticationSuccessHandler {
    type Target = BaseAuthenticationTargetUrlRequestHandler;

    fn deref(&self) -> &Self::Target {
        &self.base_authentication_target_url_request_handler
    }
}

impl DerefMut for SavedRequestAwareAuthenticationSuccessHandler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_authentication_target_url_request_handler
    }
}

impl SavedRequestAwareAuthenticationSuccessHandler {
    pub fn new() -> Self {
        Self {
            request_cache: Arc::new(HttpSessionRequestCache::default()),
            base_authentication_target_url_request_handler: Default::default(),
        }
    }

    pub fn on_authentication_success(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _authentication: &dyn Authentication,
    ) -> Option<String> {
        let save_request = self.request_cache.get_request(request, response);
        if let Some(save_request) = save_request {
            let target_url_parameter = self.get_target_url_parameter();
            if target_url_parameter.map(|s| s.is_empty()).unwrap_or(false)
                && !self.is_always_use_default_target_url()
            {
                return Some(save_request.get_redirect_url());
            }
        }

        None
    }

    pub fn set_request_cache(&mut self, request_cache: Arc<dyn RequestCache>) {
        self.request_cache = request_cache;
    }

    pub fn set_default_target_url(&mut self, default_success_url: impl ToString) {
        self.base_authentication_target_url_request_handler
            .set_default_target_url(default_success_url.to_string());
    }

    pub fn set_always_use_default_target_url(&mut self, always_use: bool) {
        self.base_authentication_target_url_request_handler
            .set_always_use_default_target_url(always_use);
    }
}

impl AuthenticationSuccessHandler for SavedRequestAwareAuthenticationSuccessHandler {
    fn on_authentication_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    ) {
        if let Some(target_url) =
            SavedRequestAwareAuthenticationSuccessHandler::on_authentication_success(
                self,
                request,
                response,
                authentication,
            )
        {
            DefaultRedirectStrategy::default().send_redirect(request, response, &target_url);
            return;
        }

        self.base_authentication_target_url_request_handler.handle(
            request,
            response,
            Some(authentication),
        );

        todo!()
    }
}
