use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use std::sync::Arc;

use crate::core::Authentication;

/// Indicates a class that is able to participate in logout handling.
/// Called by LogoutFilter.
#[async_trait]
pub trait LogoutHandler
where
    Self: Sync + Send,
{
    /// Causes a logout to be completed. The method must complete successfully.
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    );
}
