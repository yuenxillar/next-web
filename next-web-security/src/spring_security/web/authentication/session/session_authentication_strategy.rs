use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::{Authentication, authentication_error::AuthenticationError};

#[async_trait]
pub trait SessionAuthenticationStrategy
where
    Self: Send + Sync,
{
    async fn on_authentication(
        &self,
        authentication: &dyn Authentication,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError>;
}
