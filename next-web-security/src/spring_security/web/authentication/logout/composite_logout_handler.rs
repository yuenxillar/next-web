use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{core::Authentication, web::authentication::logout::LogoutHandler};

/// Performs a logout through all the LogoutHandler implementations. If any exception is thrown by
/// logout(HttpServletRequest, HttpServletResponse, Authentication), no additional
/// LogoutHandler are invoked.
#[derive(Clone)]
pub struct CompositeLogoutHandler(Vec<Arc<dyn LogoutHandler>>);

impl CompositeLogoutHandler {
    pub fn new(logout_handlers: Vec<Arc<dyn LogoutHandler>>) -> Self {
        Self(logout_handlers)
    }
}

#[async_trait]
impl LogoutHandler for CompositeLogoutHandler {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        for handler in self.0.iter() {
            handler.logout(request, response, authentication).await;
        }
    }
}
