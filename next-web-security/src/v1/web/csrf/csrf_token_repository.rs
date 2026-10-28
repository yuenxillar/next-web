use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    http::{HttpRequest, HttpResponse},
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
    ) -> Result<(), BoxError>;

    /// Loads the CSRF Token from the request.
    ///
    /// # Parameters
    /// * `request` - The HTTP request object.
    ///
    /// # Returns
    /// The loaded CSRF Token, or `None` if it does not exist.
    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>>;
}

/// Loads the CSRF token of a request.
///
/// The token is loaded with the request and the response in hand, because a
/// request without a token has one generated and stored, and the store is what
/// tells the client about the session it answers with.
///
/// # Parameters
/// * `this` - The repository the token is loaded from.
/// * `request` - The HTTP request object.
/// * `response` - The HTTP response object.
///
/// # Returns
/// A `DeferredCsrfToken` instance.
pub async fn load_deferred_token(
    this: Arc<dyn CsrfTokenRepository>,
    request: &mut dyn HttpRequest,
    response: &mut dyn HttpResponse,
) -> RepositoryDeferredCsrfToken {
    RepositoryDeferredCsrfToken::load(this, request, response).await
}
