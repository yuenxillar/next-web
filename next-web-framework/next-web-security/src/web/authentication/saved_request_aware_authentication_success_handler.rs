use std::{ops::Deref, sync::Arc};

use axum::extract::Request;

use crate::{core::authentication::Authentication, web::{authentication::{abstract_authentication_target_url_request_handler::AbstractAuthenticationTargetUrlRequestHandler, authentication_success_handler::AuthenticationSuccessHandler}, savedrequest::{http_session_request_cache::HttpSessionRequestCache, request_cache::RequestCache}}};



pub struct SavedRequestAwareAuthenticationSuccessHandler {

    request_cache: Arc<dyn RequestCache>,

    abstract_authentication_target_url_request_handler: AbstractAuthenticationTargetUrlRequestHandler,
}

impl Deref for SavedRequestAwareAuthenticationSuccessHandler {
    type Target = AbstractAuthenticationTargetUrlRequestHandler;

    fn deref(&self) -> &Self::Target {
        &self.abstract_authentication_target_url_request_handler
    }
}

impl SavedRequestAwareAuthenticationSuccessHandler {

    pub fn new() -> Self {
        Self { request_cache: HttpSessionRequestCache::new(),
            abstract_authentication_target_url_request_handler: todo!(),
         }
    }
    pub fn on_authentication_success(
        &self,
        request: &Request,
        authentication: &dyn Authentication
    ) {
        let save_request = self.request_cache.get_request(request);
        if let Some(save_request) = save_request {
            let target_url_parameter = self.get_target_url_parameter();
        }
    }

    pub fn set_request_cache(&mut self, request_cache: Arc<dyn RequestCache>) {
        self.request_cache = request_cache;
    }

    pub fn set_default_target_url(&mut self, default_success_url: impl ToString) {

    }

    pub fn set_always_use_default_target_url(&mut self, always_use: bool) {

    }
  
}


impl AuthenticationSuccessHandler for SavedRequestAwareAuthenticationSuccessHandler {
    
}