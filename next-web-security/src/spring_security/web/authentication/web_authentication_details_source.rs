use std::{any::Any, sync::Arc};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::{
    authorization::AuthenticationDetailsSource, web::authentication::WebAuthenticationDetails,
};

/// Implementation of AuthenticationDetailsSource which builds the details object from an
/// HttpRequest , creating a WebAuthenticationDetails
#[derive(Clone, Default)]
pub struct WebAuthenticationDetailsSource;

impl AuthenticationDetailsSource for WebAuthenticationDetailsSource {
    fn build_details(&self, context: &dyn HttpRequest) -> Arc<dyn Any + Send + Sync> {
        Arc::new(WebAuthenticationDetails::from(context))
    }
}
