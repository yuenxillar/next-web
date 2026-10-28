use next_web_core::{
    async_trait,
    error::BoxError,
   http::{HttpRequest, HttpResponse},
};

use std::sync::Arc;

use crate::core::Authentication;

/// Strategy that is called after a successful logout by the LogoutFilter, to handle redirection or forwarding to the appropriate destination.
/// Note that the interface is almost the same as LogoutHandler but may raise an Error.
#[async_trait]
pub trait LogoutSuccessHandler
where
    Self: Send + Sync,
{
    async fn on_logout_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) -> Result<(), BoxError>;
}
