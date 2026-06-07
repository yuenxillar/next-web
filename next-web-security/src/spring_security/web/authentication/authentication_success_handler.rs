use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::core::Authentication;

pub trait AuthenticationSuccessHandler: Send + Sync {
    fn on_authentication_success(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    );
}
