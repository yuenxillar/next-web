use next_web_core::{
    async_trait,
    error::BoxError,
   http::{HttpRequest, HttpResponse},
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
        if let Some(rd) = request.resource_dispatcher() {
            rd.forward(&self.target_url, request, response)?;
        }

        Ok(())
    }
}
