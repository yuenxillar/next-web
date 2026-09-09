use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::firewall::request_rejected_error::RequestRejectedError;

/// A request accepted and optionally normalized by an [`HttpFirewall`].
///
/// Implementations retain the original request so temporary path changes can
/// be undone by [`FirewalledRequest::reset`] before control leaves Spring
/// Security's filter chain.
pub trait FirewalledRequest: Send {
    /// Return the request view used for matching and security filters.
    fn request_mut(&mut self) -> &mut dyn HttpRequest;

    /// Restore any values temporarily changed by the firewall.
    fn reset(&mut self);
}

pub trait HttpFirewall
where
    Self: Send + Sync,
{
    /// Provides the request object which will be passed through the filter chain.
    /// RequestRejectedError if the request should be rejected immediately
    ///
    /// Validate the request without taking a mutable borrow. Splitting
    /// validation from wrapping lets a rejection handler still access the
    /// original request without unsafe aliasing.
    fn validate_request(&self, request: &dyn HttpRequest) -> Result<(), RequestRejectedError>;

    /// Wrap a request that has already passed [`Self::validate_request`].
    fn get_firewalled_request<'a>(
        &self,
        request: &'a mut dyn HttpRequest,
    ) -> Box<dyn FirewalledRequest + 'a>;

    /// Provides the response which will be passed through the filter chain.
    /// esponse the original response
    /// return either the original response or a replacement/wrapper.
    ///
    fn get_firewalled_response<'a>(
        &self,
        response: &'a mut dyn HttpResponse,
    ) -> Box<&'a mut dyn HttpResponse>;
}
