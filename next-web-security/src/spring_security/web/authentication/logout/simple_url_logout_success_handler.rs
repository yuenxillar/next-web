use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::Authentication,
    web::authentication::{
        logout::LogoutSuccessHandler, BaseAuthenticationTargetUrlRequestHandler,
    },
};

#[derive(Default)]
pub struct SimpleUrlLogoutSuccessHandler {
    inner: BaseAuthenticationTargetUrlRequestHandler,
}

#[async_trait]
impl LogoutSuccessHandler for SimpleUrlLogoutSuccessHandler {
    async fn on_logout_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) -> Result<(), BoxError> {
        self.handle(request, response, authentication)
    }
}

impl Deref for SimpleUrlLogoutSuccessHandler {
    type Target = BaseAuthenticationTargetUrlRequestHandler;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SimpleUrlLogoutSuccessHandler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
