use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{Authentication, AuthenticationError},
    web::{
        authentication::session::SessionAuthenticationStrategy,
        csrf::{CsrfTokenRepository, CsrfTokenRequestHandler},
    },
};

pub struct CsrfAuthenticationStrategy {
    token_repository: Arc<dyn CsrfTokenRepository>,
    request_handler: Option<Arc<dyn CsrfTokenRequestHandler>>,
}

impl CsrfAuthenticationStrategy {
    pub fn new(token_repository: Arc<dyn CsrfTokenRepository>) -> Self {
        Self {
            token_repository,
            request_handler: None,
        }
    }

    pub fn set_request_handler(&mut self, request_handler: &Arc<dyn CsrfTokenRequestHandler>) {
        self.request_handler = Some(request_handler.clone());
    }
}

#[async_trait]
impl SessionAuthenticationStrategy for CsrfAuthenticationStrategy {
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError> {
        todo!()
    }
}
