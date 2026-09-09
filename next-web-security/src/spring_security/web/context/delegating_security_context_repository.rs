use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use std::sync::Arc;

use crate::{
    core::context::{DeferredSecurityContext, SecurityContext},
    web::context::SecurityContextRepository,
};

#[derive(Clone)]
pub struct DelegatingSecurityContextRepository {
    delegates: Vec<Arc<dyn SecurityContextRepository>>,
}

impl DelegatingSecurityContextRepository {
    pub fn new(delegates: Vec<Arc<dyn SecurityContextRepository>>) -> Self {
        Self { delegates }
    }
}

#[async_trait]
impl SecurityContextRepository for DelegatingSecurityContextRepository {
    fn load_deferred_context(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Box<dyn DeferredSecurityContext> {
        let mut deferred_security_context: Option<Box<dyn DeferredSecurityContext>> = None;
        for delegate in self.delegates.iter() {
            let next = delegate.load_deferred_context(request);
            deferred_security_context = Some(match deferred_security_context.take() {
                Some(previous) => Box::new(DelegatingDeferredSecurityContext::new(previous, next)),
                None => next,
            });
        }

        deferred_security_context.unwrap_or_else(|| {
            panic!("DelegatingSecurityContextRepository requires at least one delegate")
        })
    }

    /// Stores the security context on completion of a request.
    /// context the non-null context which was obtained from the holder.
    async fn save_context(
        &self,
        context: &Arc<dyn SecurityContext>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        for delegate in self.delegates.iter() {
            delegate.save_context(context, request, response).await;
        }
    }

    /// Allows the repository to be queried as to whether it contains a security context
    /// for the current request.
    fn contains_context(&self, request: &mut dyn HttpRequest) -> bool {
        for delegate in self.delegates.iter() {
            if delegate.contains_context(request) {
                return true;
            }
        }

        false
    }
}

struct DelegatingDeferredSecurityContext {
    previous: Box<dyn DeferredSecurityContext>,
    next: Box<dyn DeferredSecurityContext>,
}

impl DelegatingDeferredSecurityContext {
    pub fn new(
        previous: Box<dyn DeferredSecurityContext>,
        next: Box<dyn DeferredSecurityContext>,
    ) -> Self {
        Self { previous, next }
    }
}

impl DeferredSecurityContext for DelegatingDeferredSecurityContext {
    fn get(&self) -> Arc<dyn SecurityContext> {
        let security_context = self.previous.get();
        if !self.previous.is_generated() {
            return security_context;
        }
        self.next.get()
    }

    fn is_generated(&self) -> bool {
        self.previous.is_generated() && self.next.is_generated()
    }
}
