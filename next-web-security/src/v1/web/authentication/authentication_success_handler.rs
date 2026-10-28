use next_web_core::http::{HttpRequest, HttpResponse};

use crate::core::Authentication;

pub trait AuthenticationSuccessHandler: Send + Sync {
    fn on_authentication_success(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    );
}
