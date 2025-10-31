use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{core::mgt::security_manager::SecurityManager, web::filter::access_control_filter::{AccessControlFilter, AccessControlFilterExt}};

#[derive(Clone)]
pub struct AuthorizationFilter {
    unauthorized_url: Option<String>,
    access_control_filter: AccessControlFilter,
}

impl AuthorizationFilter {
    pub fn get_unauthorized_url(&self) -> Option<&str> {
        self.unauthorized_url.as_deref()
    }

    pub fn set_unauthorized_url(&mut self, unauthorized_url: impl ToString) {
        self.unauthorized_url = Some(unauthorized_url.to_string());
    }
}

#[async_trait]
impl AccessControlFilterExt for AuthorizationFilter {
    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> bool {
        false
    }
}

impl From<Arc<dyn SecurityManager>> for AuthorizationFilter
{
    fn from(security_manager: Arc<dyn SecurityManager>) -> Self {
        Self {
            unauthorized_url: Default::default(),
            access_control_filter: AccessControlFilter::from(security_manager),
        }
    }
}
