use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    core::context::{SecurityContextHolder, SecurityContextHolderStrategy},
    web::context::SecurityContextRepository,
};

#[derive(Clone)]
pub struct SecurityContextPersistenceFilter {
    repo: Arc<dyn SecurityContextRepository>,

    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
}

impl SecurityContextPersistenceFilter {
    pub fn new(repo: Arc<dyn SecurityContextRepository>) -> Self {
        Self {
            repo,
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
        }
    }

    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    pub fn set_force_eager_session_creation(&mut self, _force_eager_session_creation: bool) {}
}

#[async_trait]
impl HttpFilter for SecurityContextPersistenceFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        filter_chain.do_filter(request, response).await
    }
}

impl Named for SecurityContextPersistenceFilter {
    fn name(&self) -> &str {
        "SecurityContextPersistenceFilter"
    }
}
