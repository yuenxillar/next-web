use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::csrf::{CsrfToken, CsrfTokenRepository};

#[derive(Default)]
pub struct CookieCsrfTokenRepository {
    cookie_http_only: bool,
}

impl CookieCsrfTokenRepository {
    pub fn with_http_only_false() -> Self {
        let mut result = CookieCsrfTokenRepository::default();
        result.cookie_http_only = false;

        result
    }
}

#[async_trait]
impl CsrfTokenRepository for CookieCsrfTokenRepository {
    async fn generate_token(&self, request: &mut dyn HttpRequest) -> Arc<dyn CsrfToken> {
        todo!()
    }

    async fn save_token(
        &self,
        token: Option<&Arc<dyn CsrfToken>>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        todo!()
    }

    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>> {
        todo!()
    }
}
