use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;
use next_web_core::util::http_method::HttpMethod;

use crate::core::mgt::security_manager::SecurityManager;
use crate::core::subject::Subject;
use crate::core::util::redirect_view::RedirectView;
use crate::web::subject::support::default_web_subject_context::DefaultWebSubjectContext;
use crate::web::subject::support::web_delegating_subject::{
    WebDelegatingSubject, DEFAULT_WEB_DELEGATING_SUBJECT_KEY,
};

pub struct WebUtils;

impl WebUtils {
    pub fn get_request_url(req: &mut dyn HttpRequest) -> String {
        let mut request_url = String::from(req.path());

        // remove duplicate leading slashes
        while request_url.len() > 1 && request_url[1..2] == *"/" {
            request_url.remove(1);
        }

        // 添加查询字符串
        if let Some(query) = req.query() {
            request_url.push('?');
            request_url.push_str(query);
        }

        request_url
    }

    pub fn redirect_to_saved_request(
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        fallback_url: &str,
    ) {
        let mut success_url = None;
        let mut context_relative = true;
        if let HttpMethod::Get = req.method() {
            success_url = Some(Self::get_request_url(req));
            context_relative = false;
        }

        if success_url.is_none() {
            success_url = Some(fallback_url.to_string());
        }
        Self::issue_redirect_with_params_and_context_elative(
            req,
            resp,
            &success_url.unwrap_or_default(),
            None,
            context_relative,
            true,
        );
    }

    pub fn issue_redirect(req: &mut dyn HttpRequest, resp: &mut dyn HttpResponse, url: &str) {
        Self::issue_redirect_with_params_and_context_elative(req, resp, url, None, true, true);
    }

    pub fn issue_redirect_with_params(
        req: &mut dyn HttpRequest,
        res: &mut dyn HttpResponse,
        url: &str,
        query_params: Option<HashMap<String, String>>,
    ) {
        Self::issue_redirect_with_params_and_context_elative(
            req,
            res,
            url,
            query_params,
            true,
            true,
        );
    }

    pub fn issue_redirect_with_params_and_context_elative(
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        url: &str,
        query_params: Option<HashMap<String, String>>,
        context_relative: bool,
        http10_compatible: bool,
    ) {
        let view =
            RedirectView::with_url_context_and_http(url, context_relative, http10_compatible);
        view.render_merged_output_model(query_params, req, resp);
    }

    pub fn get_clean_param<'a>(req: &'a dyn HttpRequest, name: &str) -> Option<&'a str> {
        req.get_parameter(name).map(|s| s.trim())
    }

    pub fn is_true(req: &dyn HttpRequest, param_name: &str) -> bool {
        let value = Self::get_clean_param(req, param_name);
        if let Some(value) = value {
            if !value.is_empty()
                && (value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("t")
                    || value.eq_ignore_ascii_case("1")
                    || value.eq_ignore_ascii_case("enabled")
                    || value.eq_ignore_ascii_case("y")
                    || value.eq_ignore_ascii_case("yes")
                    || value.eq_ignore_ascii_case("on"))
            {
                return true;
            }
        }
        false
    }

    pub fn get_subject(request: &dyn HttpRequest) -> Box<dyn Subject> {
        if let Some(subject) = request.get_attribute("NextSubject") {
            match subject {
                AnyValue::Object(obj) => {
                    if let Some(subject) = (obj as &dyn Any).downcast_ref::<WebDelegatingSubject>()
                    {
                        return Box::new(subject.clone());
                    }
                }
                _ => {}
            }
        }

        request
            .get_attribute(DEFAULT_WEB_DELEGATING_SUBJECT_KEY)
            .unwrap()
            .as_object::<Arc<dyn SecurityManager>>()
            .unwrap()
            .create_subject(Arc::new(DefaultWebSubjectContext::default()))
    }
}
