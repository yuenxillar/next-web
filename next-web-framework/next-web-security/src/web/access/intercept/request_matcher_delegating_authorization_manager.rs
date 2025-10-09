use std::sync::Arc;

use axum::extract::Request;
use next_web_core::async_trait;

use crate::authorization::authorization_decision::AuthorizationDecision;
use crate::config::web::util::matcher::request_matcher::RequestMatcher;
use crate::core::authentication::Authentication;
use crate::web::util::matcher::request_matcher_entry::RequestMatcherEntry;
use crate::authorization::authorization_manager::AuthorizationManager;
use crate::access::intercept::request_authorization_context::RequestAuthorizationContext;


pub struct RequestMatcherDelegatingAuthorizationManager {

}

#[async_trait]
impl AuthorizationManager<Request> for RequestMatcherDelegatingAuthorizationManager {
    
    async fn check(&self, authentication: Box<dyn Authentication>, request: Request) -> Option<AuthorizationDecision> {

        None
    }
}
pub struct RequestMatcherDelegatingAuthorizationManagerBuilder {
    any_request_configured: bool,
    pub(crate) mappings: Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>
}

impl RequestMatcherDelegatingAuthorizationManagerBuilder {

    pub fn add(
        &mut self,
        matcher: Box<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>
    ) {
        assert!(!self.any_request_configured, "Can't add mappings after anyRequest");
        self.mappings.push(RequestMatcherEntry::new(matcher, manager));
    }
}