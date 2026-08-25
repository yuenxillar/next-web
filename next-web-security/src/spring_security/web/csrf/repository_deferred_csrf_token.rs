use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{HttpRequestShare, HttpResponseShare},
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::csrf::{CsrfToken, CsrfTokenRepository, DeferredCsrfToken};

#[derive(Clone)]
pub struct RepositoryDeferredCsrfToken {
    csrf_token_repository: Arc<dyn CsrfTokenRepository>,
    csrf_token: Option<Arc<dyn CsrfToken>>,

    req: HttpRequestShare,
    resp: HttpResponseShare,
}

impl RepositoryDeferredCsrfToken {
    pub fn new(
        csrf_token_repository: Arc<dyn CsrfTokenRepository>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Self {
        Self {
            csrf_token_repository,
            req: request.shared().clone(),
            resp: response.shared().clone(),
            csrf_token: None,
        }
    }

    async fn init(&mut self) {
        match self.csrf_token.as_ref() {
            Some(_) => {
                return;
            }
            None => match self.csrf_token_repository.load_token(&mut self.req).await {
                Some(csrf_token) => self.csrf_token = Some(csrf_token),
                None => {
                    self.csrf_token = Some(
                        self.csrf_token_repository
                            .generate_token(&mut self.req)
                            .await,
                    );
                    self.csrf_token_repository
                        .save_token(self.csrf_token.as_ref(), &mut self.req, &mut self.resp)
                        .await;
                }
            },
        }
    }
}

#[async_trait]
impl DeferredCsrfToken for RepositoryDeferredCsrfToken {
    async fn token(&mut self) -> Arc<dyn CsrfToken> {
        self.init().await;
        self.csrf_token
            .as_ref()
            .map(Clone::clone)
            .expect("csrf_token should be initialized by now")
    }

    async fn is_generated(&mut self) -> bool {
        self.init().await;
        self.csrf_token.is_none()
    }
}
