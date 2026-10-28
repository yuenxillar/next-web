use std::{fmt::Debug, sync::Arc};

use next_web_core::{
    async_trait,
   http::{HttpRequest, HttpResponse},
};

use crate::core::{Authentication, AuthenticationError};

#[async_trait]
pub trait SessionAuthenticationStrategy
where
    Self: Send + Sync,
    Self: Debug,
{
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError>;
}
