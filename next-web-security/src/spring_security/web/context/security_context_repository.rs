use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::context::{DeferredSecurityContext, SecurityContext};

#[async_trait]
pub trait SecurityContextRepository
where
    Self: Send + Sync,
{
    fn load_deferred_context(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Box<dyn DeferredSecurityContext>;

    /// Stores the security context on completion of a request.
    /// context the non-null context which was obtained from the holder.
    async fn save_context(
        &self,
        context: &Arc<dyn SecurityContext>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    );

    /// Allows the repository to be queried as to whether it contains a security context
    /// for the current request.
    fn contains_context(&self, request: &mut dyn HttpRequest) -> bool;
}
