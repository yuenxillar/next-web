use std::sync::Arc;

use crate::core::mgt::security_manager::SecurityManager;
use crate::core::subject::support::default_subject_context::DefaultSubjectContext;
use crate::core::util::object::Object;
use crate::{
    core::{
        subject::Subject,
        util::{ant_path_matcher::AntPathMatcher, pattern_matcher::PatternMatcher, web::WebUtils},
    },
    web::filter::path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
};
use next_web_core::async_trait;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

#[derive(Clone)]
pub struct AccessControlFilter<T = AntPathMatcher> {
    login_url: String,

    security_manager: Arc<dyn SecurityManager>,
    // web_security_context: WebSecurityContext,
    path_matching_filter: PathMatchingFilter<T>,
}

impl<T> AccessControlFilter<T> {
    pub const DEFAULT_LOGIN_URL: &'static str = "/login";

    pub fn get_login_url(&self) -> &str {
        &self.login_url
    }

    pub fn set_login_url<S: ToString>(&mut self, login_url: S) {
        self.login_url = login_url.to_string();
    }

    pub fn get_subject(
        &self,
        _request: &dyn HttpRequest,
        _response: &dyn HttpResponse,
    ) -> Box<dyn Subject> {
        self.security_manager
            .create_subject(Arc::new(DefaultSubjectContext::default()))
    }

    pub fn save_request_and_redirect_to_login(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        subject: &dyn Subject,
    ) {
        self.save_request(request, subject);
        self.redirect_to_login(request, response);
    }

    pub fn save_request(&self, request: &dyn HttpRequest, subject: &dyn Subject) {
        WebUtils::save_request(request, subject);
    }

    pub fn redirect_to_login(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        let login_url = self.get_login_url();
        WebUtils::issue_redirect(request, response, login_url);
    }
}

impl<T> From<Arc<dyn SecurityManager>> for AccessControlFilter<T>
where
    T: PatternMatcher + Default,
{
    fn from(security_manager: Arc<dyn SecurityManager>) -> Self {
        Self {
            login_url: String::from("/login"),
            path_matching_filter: Default::default(),
            security_manager,
        }
    }
}

#[async_trait]
impl PathMatchingFilterExt for AccessControlFilter {
    async fn on_pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: &Object,
    ) -> bool {
        self.is_access_allowed(request, response, mapped_value)
            || self.on_access_denied(request, response).await
    }
}

impl AccessControlFilterExt for AccessControlFilter {
    fn is_login_request(&self, request: &dyn HttpRequest, _response: &dyn HttpResponse) -> bool {
        self.path_matching_filter
            .paths_match(self.get_login_url(), request)
    }
}

#[allow(unused_variables)]
#[async_trait]
pub trait AccessControlFilterExt: Send + Sync {
    fn is_access_allowed(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        mapped_value: &Object,
    ) -> bool {
        false
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> bool {
        false
    }

    fn is_login_request(&self, request: &dyn HttpRequest, response: &dyn HttpResponse) -> bool {
        false
    }
}
