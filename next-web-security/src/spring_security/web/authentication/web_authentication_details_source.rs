use std::sync::Arc;

use next_web_core::http::HttpRequest;

use crate::{
    authentication::authentication_details_source::AuthenticationDetailsSource,
    web::authentication::{Identity, WebAuthenticationDetails},
};

/// Implementation of AuthenticationDetailsSource which builds the details object from an
/// HttpRequest , creating a WebAuthenticationDetails
#[derive(Clone, Default)]
pub struct WebAuthenticationDetailsSource;

impl AuthenticationDetailsSource for WebAuthenticationDetailsSource {
    fn build_details(&self, context: &dyn HttpRequest) -> Arc<dyn Identity> {
        Arc::new(WebAuthenticationDetails::from(context))
    }
}
