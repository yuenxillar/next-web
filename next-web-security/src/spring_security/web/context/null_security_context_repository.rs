use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::context::{
        DeferredSecurityContext, SecurityContext, SecurityContextHolder,
        SecurityContextHolderStrategy,
    },
    web::context::{SecurityContextRepository, SuppliedDeferredSecurityContext},
};

#[derive(Clone)]
pub struct NullSecurityContextRepository {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
}

impl NullSecurityContextRepository {
    /// Sets the SecurityContextHolderStrategy to use. The default action is to use the
    /// SecurityContextHolderStrategy stored in SecurityContextHolder.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }
}

#[async_trait]
impl SecurityContextRepository for NullSecurityContextRepository {
    #[allow(unused_variables)]
    fn contains_context(&self, request: &mut dyn HttpRequest) -> bool {
        false
    }

    #[allow(unused_variables)]
    async fn save_context(
        &self,
        context: &Arc<dyn SecurityContext>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
    }

    // #[allow(unused_variables)]
    fn load_deferred_context(
        &self,
        _request: &mut dyn HttpRequest,
    ) -> Box<dyn DeferredSecurityContext> {
        Box::new(SuppliedDeferredSecurityContext::new(
            None,
            self.security_context_holder_strategy.to_owned(),
        ))
    }
}

impl Default for NullSecurityContextRepository {
    fn default() -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
        }
    }
}
