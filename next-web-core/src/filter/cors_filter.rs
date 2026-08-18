use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    cors::{CorsConfigurationSource, CorsProcessor, CorsUtils, DefaultCorsProcessor},
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

///
/// HttpFilter to handle CORS pre-flight requests and intercept CORS simple and actual
/// requests with a CorsProcessor, and to update the response, for example, with CORS response headers,
/// based on the policy matched through the provided CorsConfigurationSource.
#[derive(Clone)]
pub struct CorsFilter {
    config_source: Arc<dyn CorsConfigurationSource>,
    processor: Arc<dyn CorsProcessor>,
}

impl CorsFilter {
    /// Constructor accepting a CorsConfigurationSource used by the filter to find the CorsConfiguration to use for each incoming request.
    pub fn new(config_source: Arc<dyn CorsConfigurationSource>) -> Self {
        let processor = Arc::new(DefaultCorsProcessor::default());
        Self {
            config_source,
            processor,
        }
    }

    /// Configure a custom CorsProcessor to use to apply the matched CorsConfiguration for a request.
    pub fn set_processor(&mut self, processor: Arc<dyn CorsProcessor>) {
        self.processor = processor;
    }
}

#[async_trait]
impl HttpFilter for CorsFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let cors_configuration = self.config_source.cors_configuration(request);

        let is_valid = self
            .processor
            .process_request(cors_configuration, request, response)?;

        if !is_valid || CorsUtils::is_pre_flight_request(request) {
            return Ok(());
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for CorsFilter {
    fn name(&self) -> &str {
        "CorsFilter"
    }
}
