use std::any::Any;
use std::ops::Deref;

use crate::core::util::object::Object;
use crate::web::subject::support::web_delegating_subject::WebDelegatingSubject;
use crate::{
    core::{
        subject::Subject,
        util::{ant_path_matcher::AntPathMatcher, web::WebUtils},
    },
    web::filter::path_matching_filter::PathMatchingFilter,
};
use next_web_core::anys::any_value::AnyValue;
use next_web_core::async_trait;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};
use tracing::debug;

#[derive(Clone)]
pub struct AccessControlFilter<T = AntPathMatcher> {
    login_url: String,
    pub(crate) path_matching_filter: PathMatchingFilter<T>,
}

impl<T> AccessControlFilter<T> {
    pub const DEFAULT_LOGIN_URL: &'static str = "/login.html";
    pub const GET_METHOD: &'static str = "GET";
    pub const POST_METHOD: &'static str = "POST";

    pub fn get_login_url(&self) -> &str {
        &self.login_url
    }

    pub fn set_login_url<S: ToString>(&mut self, login_url: S) {
        self.login_url = login_url.to_string();
    }

    pub async fn get_subject(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Box<dyn Subject> {
        WebUtils::get_subject(req, resp).await
    }

    pub fn save_request_and_redirect_to_login(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        if let Some(subject) = request.get_attribute("NextSubject") {
            match subject {
                AnyValue::Object(obj) => {
                    if let Some(_subject) = (obj as &dyn Any).downcast_ref::<WebDelegatingSubject>()
                    {
                        debug!("Subject found in request attribute");
                        // self.save_request(request, subject);
                    }
                }
                _ => {}
            }
        }
        self.redirect_to_login(request, response);
    }

    pub fn redirect_to_login(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        let login_url = self.get_login_url();
        WebUtils::issue_redirect(request, response, login_url);
    }

    pub fn process_path_config(&mut self, path: &str, config: &str) {
        self.path_matching_filter.process_path_config(path, config);
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
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        false
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        false
    }

    fn is_login_request(&self, request: &dyn HttpRequest, response: &dyn HttpResponse) -> bool {
        false
    }
}

impl Deref for AccessControlFilter {
    type Target = PathMatchingFilter;

    fn deref(&self) -> &Self::Target {
        &self.path_matching_filter
    }
}

impl Default for AccessControlFilter {
    fn default() -> Self {
        Self {
            login_url: String::from("/login"),
            // security_manager: Default::default(),
            path_matching_filter: Default::default(),
        }
    }
}
