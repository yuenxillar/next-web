use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::{authentication_error::AuthenticationError, Authentication};

#[async_trait]
pub trait SessionAuthenticationStrategy
where
    Self: Send + Sync,
{
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError>;
}
