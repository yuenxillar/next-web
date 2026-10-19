//! The single logout endpoints of the service provider.

mod saml2_logout_request_filter;
mod saml2_logout_response_filter;

pub use saml2_logout_request_filter::{DEFAULT_LOGOUT_REQUEST_URL, Saml2LogoutRequestFilter};
pub use saml2_logout_response_filter::{DEFAULT_LOGOUT_RESPONSE_URL, Saml2LogoutResponseFilter};
