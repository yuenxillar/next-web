use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

/// Encapsulates the redirection logic for all classes in the framework which perform redirects.
pub trait RedirectStrategy
where
    Self: Send + Sync,
{
    /// Performs a redirect to the supplied URL
    ///
    /// # Arguments
    ///
    /// * `request` - The HTTP request
    /// * `response` - The HTTP response
    /// * `url` - The target URL to redirect to, for example "/login"
    fn send_redirect(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        url: &str,
    ) -> Result<(), BoxError>;
}
