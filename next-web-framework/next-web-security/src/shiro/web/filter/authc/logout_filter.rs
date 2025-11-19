use std::ops::{Deref, DerefMut};

use axum::http::StatusCode;
use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
    util::http_method::HttpMethod,
};
use tracing::error;

use crate::{
    core::util::web::WebUtils,
    web::filter::{
        advice_filter::{AdviceFilter, AdviceFilterExt},
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::PathMatchingFilterExt,
    },
};

#[derive(Clone)]
pub struct LogoutFilter {
    redirect_url: String,
    post_only_logout: bool,

    advice_filter: AdviceFilter,
}

impl LogoutFilter {
    const DEFAULT_REDIRECT_URL: &str = "/";

    pub fn is_post_only_logout(&self) -> bool {
        self.post_only_logout
    }

    pub fn set_post_only_logout(&mut self, post_only_logout: bool) {
        self.post_only_logout = post_only_logout
    }

    pub fn set_redirect_url(&mut self, redirect_url: impl ToString) {
        self.redirect_url = redirect_url.to_string();
    }

    pub fn get_redirect_url(&self) -> &str {
        &self.redirect_url
    }

    fn on_logout_request_not_a_post(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> bool {
        response.set_status_code(StatusCode::METHOD_NOT_ALLOWED);
        response.insert_header("Allow".as_bytes(), "POST");

        false
    }

    fn issue_redirect(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        redirect_url: &str,
    ) {
        WebUtils::issue_redirect(request, response, redirect_url);
    }
}

#[async_trait]
impl AdviceFilterExt for LogoutFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        let mut subject = WebUtils::get_subject(request);

        // Check if POST only logout is enabled
        if self.is_post_only_logout() {
            // check if the current request's method is a POST, if not redirect
            if !(request.method() == HttpMethod::Post) {
                return self.on_logout_request_not_a_post(request, response);
            }
        }

        let redirect_url = self.get_redirect_url();
        // added for SHIRO-298:

        if let Err(ise) = subject.logout().await {
            error!("Encountered session errror during logout.  This can generally safely be ignored: {}", ise);
        }

        self.issue_redirect(request, response, redirect_url);
        false
    }
}

impl Required<OncePerRequestFilter> for LogoutFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self.advice_filter.once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self.advice_filter.once_per_request_filter
    }
}

impl Named for LogoutFilter {
    fn name(&self) -> &str {
        "LogoutFilter"
    }
}

impl Deref for LogoutFilter {
    type Target = AdviceFilter;

    fn deref(&self) -> &Self::Target {
        &self.advice_filter
    }
}

impl DerefMut for LogoutFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.advice_filter
    }
}

impl Default for LogoutFilter {
    fn default() -> Self {
        Self {
            redirect_url: Self::DEFAULT_REDIRECT_URL.to_string(),
            advice_filter: Default::default(),
            post_only_logout: Default::default(),
        }
    }
}
