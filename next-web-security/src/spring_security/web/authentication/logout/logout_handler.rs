use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::Authentication;

#[async_trait]
pub trait LogoutHandler
where
    Self: Sync + Send,
{
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&dyn Authentication>,
    );
}
