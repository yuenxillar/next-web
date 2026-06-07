use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::Authentication,
    web::{authentication::logout::LogoutHandler, csrf::CsrfTokenRepository},
};

#[derive(Clone)]
pub struct CsrfLogoutHandler {
    csrf_token_repository: Arc<dyn CsrfTokenRepository>,
}

impl CsrfLogoutHandler {
    pub fn new(csrf_token_repository: Arc<dyn CsrfTokenRepository>) -> Self {
        Self {
            csrf_token_repository,
        }
    }
}

#[async_trait]
impl LogoutHandler for CsrfLogoutHandler {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&dyn Authentication>,
    ) {
        self.csrf_token_repository
            .save_token(None, request, response)
            .await;
    }
}
