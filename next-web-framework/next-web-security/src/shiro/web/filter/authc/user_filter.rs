use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::{
        access_control_filter::{AccessControlFilter, AccessControlFilterExt},
        advice_filter::AdviceFilterExt,
        once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct UserFilter {
    access_control_filter: AccessControlFilter,
}

#[async_trait]
impl AccessControlFilterExt for UserFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        if self.is_login_request(request, response) {
            return true;
        } else {
            let subject = WebUtils::get_subject(request);

            // If principal is not null, then the user is known and should be allowed access.
            return subject.get_principal().await.is_some();
        }
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        self.save_request_and_redirect_to_login(request, response);

        false
    }
}

#[async_trait]
impl AdviceFilterExt for UserFilter {}

impl Required<OncePerRequestFilter> for UserFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }
}

impl Named for UserFilter {
    fn name(&self) -> &str {
        "UserFilter"
    }
}

impl Deref for UserFilter {
    type Target = AccessControlFilter;

    fn deref(&self) -> &Self::Target {
        &self.access_control_filter
    }
}

impl DerefMut for UserFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.access_control_filter
    }
}

impl Default for UserFilter {
    fn default() -> Self {
        Self {
            access_control_filter: Default::default(),
        }
    }
}
