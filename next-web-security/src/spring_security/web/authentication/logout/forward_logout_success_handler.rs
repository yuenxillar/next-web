use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use std::sync::Arc;

use crate::{
    core::Authentication,
    web::{authentication::logout::LogoutSuccessHandler, util::UrlUtils},
};

#[derive(Clone)]
pub struct ForwardLogoutSuccessHandler {
    target_url: String,
}

impl ForwardLogoutSuccessHandler {
    pub fn new<T>(target_url: T) -> Self
    where
        T: Into<String>,
    {
        let target_url = target_url.into();
        assert!(
            UrlUtils::is_valid_redirect_url(target_url.as_str()),
            "{} is not a valid target URL",
            &target_url
        );
        Self { target_url }
    }
}

#[async_trait]
impl LogoutSuccessHandler for ForwardLogoutSuccessHandler {
    async fn on_logout_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _authentication: Option<&Arc<dyn Authentication>>,
    ) -> Result<(), BoxError> {
        if let Some(rd) = request.request_dispatcher(&self.target_url) {
            rd.forward(request, response)?;
        }

        Ok(())
    }
}
