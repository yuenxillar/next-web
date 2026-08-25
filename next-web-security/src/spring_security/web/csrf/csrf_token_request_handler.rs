use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::csrf::{CsrfTokenRequestResolver, DeferredCsrfToken};

/// A callback interface that is used to make the CsrfToken created by the CsrfTokenRepository
/// available as a request attribute. Implementations of this interface may choose to perform additional
/// tasks or customize how the token is made available to the application through request attributes.
#[async_trait]
pub trait CsrfTokenRequestHandler
where
    Self: CsrfTokenRequestResolver,
    Self: Send + Sync,
{
    /// Handles a request using a CsrfToken.
    async fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        csrf_token: &mut dyn DeferredCsrfToken,
    );
}
