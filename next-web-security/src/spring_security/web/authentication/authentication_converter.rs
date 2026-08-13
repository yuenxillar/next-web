use std::any::Any;

use crate::core::{Authentication, AuthenticationError};
use next_web_core::{clone_trait_object, traits::http::http_request::HttpRequest, DynClone};

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
    Self: Any + DynClone,
{
    /// Converts the `HttpRequest` into an `Authentication` or returns `None`
    /// if no authentication attempt should be made.
    fn convert(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

clone_trait_object!(AuthenticationConverter where Self:  Send + Sync);
