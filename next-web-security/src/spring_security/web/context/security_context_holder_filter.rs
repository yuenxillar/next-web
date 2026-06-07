use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    core::context::{
        security_context_holder::SecurityContextHolder,
        security_context_holder_strategy::SecurityContextHolderStrategy,
    },
    web::context::SecurityContextRepository,
};

#[derive(Clone)]
pub struct SecurityContextHolderFilter {
    security_context_repository: Arc<dyn SecurityContextRepository>,
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
}

impl SecurityContextHolderFilter {
    const FILTER_APPLIED: &str = "SecurityContextHolderFilter.APPLIED";

    pub fn new(security_context_repository: Arc<dyn SecurityContextRepository>) -> Self {
        Self {
            security_context_repository,
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
        }
    }
}

#[async_trait]
impl HttpFilter for SecurityContextHolderFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        if request.get_attribute(Self::FILTER_APPLIED).is_some() {
            return filter_chain.do_filter(request, response).await;
        }

        request.set_attribute(Self::FILTER_APPLIED, AnyValue::Boolean(true));

        let context = self.security_context_repository.load_context(request);

        self.security_context_holder_strategy
            .scope_with_context(
                context,
                Box::pin(async {
                    filter_chain.do_filter(request, response).await?;
                    request.remove_attribute(Self::FILTER_APPLIED);

                    Ok(())
                }),
            )
            .await
    }
}

impl Named for SecurityContextHolderFilter {
    fn name(&self) -> &str {
        "SecurityContextHolderFilter"
    }
}
