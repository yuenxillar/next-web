use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::csrf::{CsrfToken, CsrfTokenRequestHandler, CsrfTokenRequestResolver};

#[derive(Clone, Default)]
pub struct XorCsrfTokenRequestAttributeHandler {}

impl CsrfTokenRequestHandler for XorCsrfTokenRequestAttributeHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        csrf_token: &dyn Fn() -> Arc<dyn CsrfToken>,
    ) {
        todo!()
    }
}

impl CsrfTokenRequestResolver for XorCsrfTokenRequestAttributeHandler {
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn CsrfToken,
    ) -> Option<String> {
        todo!()
    }
}
