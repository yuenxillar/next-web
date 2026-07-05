use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::csrf::{repository_deferred_csrf_token::RepositoryDeferredCsrfToken, CsrfToken};

#[async_trait]
pub trait CsrfTokenRepository
where
    Self: Send + Sync,
{
    /// Generates a new CSRF Token.
    ///
    /// # Parameters
    /// * `request` - The HTTP request object.
    ///
    /// # Returns
    /// The generated CSRF Token.
    async fn generate_token(&self, request: &mut dyn HttpRequest) -> Arc<dyn CsrfToken>;

    /// Saves the CSRF Token.
    ///
    /// If the token is `None`, the token will be deleted.
    ///
    /// # Parameters
    /// * `token` - The CSRF Token to save, or `None` to delete.
    /// * `request` - The HTTP request object.
    /// * `response` - The HTTP response object.
    async fn save_token(
        &self,
        token: Option<&Arc<dyn CsrfToken>>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    );

    /// Loads the CSRF Token from the request.
    ///
    /// # Parameters
    /// * `request` - The HTTP request object.
    ///
    /// # Returns
    /// The loaded CSRF Token, or `None` if it does not exist.
    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>>;
}

/// Lazily loads a CSRF Token.
///
/// Returns a `DeferredCsrfToken` that caches the token to avoid repeated loading.
///
/// # Parameters
/// * `request` - The HTTP request object.
/// * `response` - The HTTP response object.
///
/// # Returns
/// A `DeferredCsrfToken` instance.
#[allow(dead_code)]
pub fn load_deferred_token(_self: Arc<dyn CsrfTokenRepository>) -> RepositoryDeferredCsrfToken {
    RepositoryDeferredCsrfToken::new(_self)
}
