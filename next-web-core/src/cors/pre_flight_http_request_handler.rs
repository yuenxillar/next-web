use std::sync::Arc;

use crate::{
    cors::{CorsConfiguration, CorsProcessor, DefaultCorsProcessor, PreFlightRequestHandler},
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

#[derive(Clone)]
pub struct PreFlightHttpRequestHandler {
    cors_processor: Arc<dyn CorsProcessor>,
    config: Option<CorsConfiguration>,
}

impl PreFlightHttpRequestHandler {
    pub fn new<T>(cors_processor: T, config: Option<CorsConfiguration>) -> Self
    where
        T: CorsProcessor + 'static,
    {
        Self {
            cors_processor: Arc::new(cors_processor),
            config,
        }
    }

    pub fn with_config(config: Option<CorsConfiguration>) -> Self {
        Self {
            cors_processor: Arc::new(DefaultCorsProcessor::default()),
            config,
        }
    }
}

impl PreFlightRequestHandler for PreFlightHttpRequestHandler {
    fn handle_pre_flight(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        self.cors_processor
            .process_request(self.config.as_ref(), request, response)?;

        Ok(())
    }
}
