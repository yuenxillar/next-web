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
        let mut deferred_security_context = None;
        for delegate in self.delegates.iter() {
            if let Some(_dsc) = deferred_security_context.take() {
                let dsc: Box<dyn DeferredSecurityContext> =
                    Box::new(DelegatingDeferredSecurityContext::new(
                        _dsc,
                        delegate.load_deferred_context(request),
                    ));
                deferred_security_context = Some(dsc);
            }
        }

        deferred_security_context.unwrap_or(self.delegates[0].load_deferred_context(request))
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
    fn get(&mut self) -> Option<Arc<dyn SecurityContext>> {
        let security_context = self.previous.get();
        if !self.previous.is_generated() {
            return security_context;
        }
        self.next.get()
    }

    fn is_generated(&mut self) -> bool {
        self.previous.is_generated() && self.next.is_generated()
    }
}
