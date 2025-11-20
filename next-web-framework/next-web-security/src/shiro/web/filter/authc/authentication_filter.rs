use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::access_control_filter::{AccessControlFilter, AccessControlFilterExt},
};

#[derive(Clone)]
pub struct AuthenticationFilter {
    success_url: String,
    pub(crate) access_control_filter: AccessControlFilter,
}

impl AuthenticationFilter {
    pub const DEFAULT_SUCCESS_URL: &'static str = "/";

    pub fn get_success_url(&self) -> &str {
        &self.success_url
    }

    pub fn set_success_url(&mut self, success_url: impl ToString) {
        self.success_url = success_url.to_string();
    }

    pub fn issue_success_redirect(&self, req: &mut dyn HttpRequest, res: &mut dyn HttpResponse) {
        WebUtils::redirect_to_saved_request(req, res, self.get_success_url());
    }
}

#[async_trait]
impl AccessControlFilterExt for AuthenticationFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        let subject = self.access_control_filter.get_subject(request).await;
        subject.is_authenticated().await && subject.get_principal().await.is_some()
    }
}

impl Deref for AuthenticationFilter {
    type Target = AccessControlFilter;

    fn deref(&self) -> &Self::Target {
        &self.access_control_filter
    }
}

impl DerefMut for AuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.access_control_filter
    }
}

impl Default for AuthenticationFilter {
    fn default() -> Self {
        Self {
            success_url: Default::default(),
            access_control_filter: Default::default(),
        }
    }
}
