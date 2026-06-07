use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{core::Authentication, web::authentication::remember_me_services::RememberMeServices};

/// Base implementation of `RememberMeServices`.
/// Subclasses should override `auto_login` with actual token processing logic.
#[derive(Clone)]
pub struct AbstractRememberMeServices {}

impl AbstractRememberMeServices {
    pub fn get_parameter(&self) -> &str {
        "remember-me"
    }
}

impl RememberMeServices for AbstractRememberMeServices {
    fn auto_login(
        &self,
        _request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn Authentication>> {
        // Default: no remember-me token handling.
        // Override in concrete implementations (e.g. TokenBasedRememberMeServices).
        None
    }
}
