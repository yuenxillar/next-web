use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::authentication::AuthPrincipal;

/// Provides an authentication `details` object for a given web request.
pub trait AuthenticationDetailsSource<T = AuthPrincipal>
where
    Self: Send + Sync,
{
    /// Called by a class when it wishes a new authentication details instance to be created.
    fn build_details(&self, context: &dyn HttpRequest) -> T
    where
        T: 'static;
}
