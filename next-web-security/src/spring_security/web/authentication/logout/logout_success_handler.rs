use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::Authentication;

#[async_trait]
pub trait LogoutSuccessHandler
where
    Self: Send + Sync,
{
    async fn on_logout_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&dyn Authentication>,
    ) -> Result<(), BoxError>;
}
