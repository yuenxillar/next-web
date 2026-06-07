use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::csrf::{CsrfToken, CsrfTokenRepository, DeferredCsrfToken};

pub struct RepositoryDeferredCsrfToken {
    csrf_token_repository: Arc<dyn CsrfTokenRepository>,
    csrf_token: Option<Arc<dyn CsrfToken>>,
}

impl RepositoryDeferredCsrfToken {
    pub fn new(csrf_token_repository: Arc<dyn CsrfTokenRepository>) -> Self {
        Self {
            csrf_token_repository,
            csrf_token: None,
        }
    }

    async fn init(&mut self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse) {
        match self.csrf_token.as_ref() {
            Some(_) => {
                return;
            }
            None => match self.csrf_token_repository.load_token(request).await {
                Some(csrf_token) => self.csrf_token = Some(csrf_token),
                None => {
                    self.csrf_token =
                        Some(self.csrf_token_repository.generate_token(request).await);
                    self.csrf_token_repository
                        .save_token(self.csrf_token.as_ref(), request, response)
                        .await;
                }
            },
        }
    }
}

impl DeferredCsrfToken for RepositoryDeferredCsrfToken {
    fn get_token(&self) -> &dyn super::CsrfToken {
        self.csrf_token.as_deref().unwrap()
    }

    fn is_generated(&self) -> bool {
        self.csrf_token.is_none()
    }
}
