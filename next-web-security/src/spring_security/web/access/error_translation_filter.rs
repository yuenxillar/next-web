use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationTrustResolver,
    web::{
        access::AccessDeniedHandler, authentication_entry_point::AuthenticationEntryPoint,
        savedrequest::RequestCache,
    },
};

#[derive(Clone)]
pub struct ErrorTranslationFilter {
    access_denied_handler: Arc<dyn AccessDeniedHandler>,
    authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    authentication_trust_resolver: Arc<dyn AuthenticationTrustResolver>,

    request_cache: Arc<dyn RequestCache>,
}

#[async_trait]
impl HttpFilter for ErrorTranslationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        let result = filter_chain.do_filter(request, response).await;
        Ok(())
    }
}

impl Named for ErrorTranslationFilter {
    fn name(&self) -> &str {
        "ErrorTranslationFilter"
    }
}
