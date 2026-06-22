use crate::core::Authentication;
use next_web_core::traits::http::http_request::HttpRequest;

/// A strategy used to convert from an `HttpRequest` to an `Authentication`
/// of a particular type. Used to authenticate with an appropriate
/// `AuthenticationManager`.
///
/// If the result is `None`, then it is assumed that the converter has no
/// authentication to submit. The `AuthenticationFilter` will then continue
/// the filter chain.
///

pub trait AuthenticationConverter
where
    Self: Send + Sync,
{
    /// Converts the `HttpRequest` into an `Authentication` or returns `None`
    /// if no authentication attempt should be made.
    fn convert(&self, request: &dyn HttpRequest) -> Option<Box<dyn Authentication>>;
}
