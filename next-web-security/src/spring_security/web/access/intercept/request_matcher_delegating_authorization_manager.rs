use std::sync::Arc;

use next_web_core::async_trait;

use crate::access::intercept::request_authorization_context::RequestAuthorizationContext;
use crate::authorization::authorization_decision::AuthorizationDecision;
use crate::authorization::authorization_manager::AuthorizationManager;
use crate::config::web::util::matcher::request_matcher::RequestMatcher;
use crate::core::authentication::Authentication;
use crate::web::util::matcher::request_matcher_entry::RequestMatcherEntry;

#[derive(Clone)]
pub struct RequestMatcherDelegatingAuthorizationManager {
    mappings: Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>,
}

impl RequestMatcherDelegatingAuthorizationManager {
    pub fn new(
        mappings: Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>,
    ) -> Self {
        Self { mappings }
    }
}

#[async_trait]
impl AuthorizationManager<RequestAuthorizationContext> for RequestMatcherDelegatingAuthorizationManager {
    async fn check(
        &self,
        authentication: Box<dyn Authentication>,
        request: RequestAuthorizationContext,
    ) -> Option<AuthorizationDecision> {
        let candidate = axum::http::Request::builder()
            .method(request.method())
            .uri(request.path())
            .body(axum::body::Body::empty())
            .expect("failed to create request authorization candidate");

        for entry in &self.mappings {
            if entry.request_matcher().matches(&candidate) {
                return entry
                    .entry()
                    .check(authentication, request.clone())
                    .await;
            }
        }

        None
    }
}

#[derive(Clone)]
pub struct RequestMatcherDelegatingAuthorizationManagerBuilder {
    any_request_configured: bool,
    pub(crate) mappings:
        Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>,
}

impl RequestMatcherDelegatingAuthorizationManagerBuilder {
    pub fn add(
        &mut self,
        matcher: Box<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        assert!(
            !self.any_request_configured,
            "Can't add mappings after anyRequest"
        );
        self.mappings
            .push(RequestMatcherEntry::new(matcher, manager));
    }

    pub fn build(
        &self,
    ) -> Arc<dyn AuthorizationManager<RequestAuthorizationContext>> {
        Arc::new(RequestMatcherDelegatingAuthorizationManager::new(
            self.mappings.clone(),
        ))
    }
}

impl Default for RequestMatcherDelegatingAuthorizationManagerBuilder {
    fn default() -> Self {
        Self {
            any_request_configured: false,
            mappings: Vec::new(),
        }
    }
}
