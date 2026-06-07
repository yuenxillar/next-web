use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

#[derive(Clone)]
pub struct ErrorTranslationFilter {}

#[async_trait]
impl HttpFilter for ErrorTranslationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}

impl Named for ErrorTranslationFilter {
    fn name(&self) -> &str {
        "ErrorTranslationFilter"
    }
}
