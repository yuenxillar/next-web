use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{debug, enabled, Level};

/// Eagerly creates HttpSession if it does not already exist.
#[derive(Clone, Default)]
pub struct ForceEagerSessionCreationFilter;

#[async_trait]
impl HttpFilter for ForceEagerSessionCreationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if let Some(session) = request.session() {
            if enabled!(Level::DEBUG) && session.is_new() {
                debug!("Created session eagerly");
            }
        }
        filter_chain.do_filter(request, response).await
    }
}

impl Named for ForceEagerSessionCreationFilter {
    fn name(&self) -> &str {
        "ForceEagerSessionCreationFilter"
    }
}
