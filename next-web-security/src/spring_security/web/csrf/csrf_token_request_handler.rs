use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::csrf::{CsrfToken, CsrfTokenRequestResolver};

pub trait CsrfTokenRequestHandler
where
    Self: CsrfTokenRequestResolver,
    Self: Send + Sync,
{
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        csrf_token: &dyn Fn() -> Arc<dyn CsrfToken>,
    );
}
