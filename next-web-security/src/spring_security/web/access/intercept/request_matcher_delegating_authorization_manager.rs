use std::sync::Arc;

use next_web_core::async_trait;

use crate::access::intercept::request_authorization_context::RequestAuthorizationContext;
use crate::authorization::AuthorizationDecision;
use crate::authorization::AuthorizationManager;
use crate::authorization::AuthorizationResult;
use crate::core::Authentication;
use crate::web::util::matcher::RequestMatcher;
use crate::web::util::matcher::RequestMatcherEntry;

#[derive(Clone)]
pub struct RequestMatcherDelegatingAuthorizationManager {
    mappings: Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>,
}

impl RequestMatcherDelegatingAuthorizationManager {
    pub fn new(
        mappings: Vec<
            RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>,
        >,
    ) -> Self {
        Self { mappings }
    }
}

#[async_trait]
impl AuthorizationManager<RequestAuthorizationContext>
    for RequestMatcherDelegatingAuthorizationManager
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut RequestAuthorizationContext,
    ) -> Option<Box<dyn AuthorizationResult>> {
        // let candidate = axum::http::Request::builder()
        //     .method(request.method())
        //     .uri(request.path())
        //     .body(axum::body::Body::empty())
        //     .expect("failed to create request authorization candidate");

        // for entry in &self.mappings {
        //     if entry.request_matcher().matches(todo!()) {
        //         return entry.entry().check(authentication, request.clone()).await;
        //     }
        // }

        // None
        todo!()
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
        matcher: Arc<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        assert!(
            !self.any_request_configured,
            "Can't add mappings after anyRequest"
        );
        self.mappings
            .push(RequestMatcherEntry::new(matcher, manager));
    }

    pub fn build(&mut self) -> Arc<dyn AuthorizationManager<RequestAuthorizationContext>> {
        Arc::new(RequestMatcherDelegatingAuthorizationManager::new(
            std::mem::take(&mut self.mappings),
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
