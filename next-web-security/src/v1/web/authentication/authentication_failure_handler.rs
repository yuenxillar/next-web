use std::fmt::Debug;

use next_web_core::{
    error::BoxError,
   http::{HttpRequest, HttpResponse},
};

use crate::core::AuthenticationError;

pub trait AuthenticationFailureHandler
where
    Self: Send + Sync,
    Self: Debug,
{
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    ) -> Result<(), BoxError>;
}
