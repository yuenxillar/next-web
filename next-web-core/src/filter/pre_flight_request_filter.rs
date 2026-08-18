use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    cors::{CorsUtils, PreFlightRequestHandler},
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

/// Servlet filter that handles pre-flight requests through a
/// `PreFlightRequestHandler` and bypasses the rest of the chain.
#[derive(Clone)]
pub struct PreFlightRequestFilter {
    handler: Arc<dyn PreFlightRequestHandler>,
}

impl PreFlightRequestFilter {
    pub fn new(handler: Arc<dyn PreFlightRequestHandler>) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl HttpFilter for PreFlightRequestFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if !CorsUtils::is_pre_flight_request(request) {
            return filter_chain.do_filter(request, response).await;
        }
        self.handler.handle_pre_flight(request, response)?;
        Ok(())
    }
}

impl Named for PreFlightRequestFilter {
    fn name(&self) -> &str {
        "PreFlightRequestFilter"
    }
}