use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use next_web_core::any::AnyValue;
use next_web_core::http::HttpMethod;
use next_web_core::http::{HttpRequest, HttpResponse};

use crate::core::mgt::security_manager::SecurityManager;
use crate::core::subject::Subject;
use crate::core::util::redirect_view::RedirectView;
use crate::web::subject::support::default_web_subject_context::DefaultWebSubjectContext;
use crate::web::subject::support::web_delegating_subject::{
    DEFAULT_WEB_DELEGATING_SUBJECT_KEY, WebDelegatingSubject,
};

pub struct WebUtils;

impl WebUtils {
    /// The request attribute that records that the identity of a request was
    /// removed, so a remember-me cookie is not written again on the way out.
    ///
    /// The name is the one Apache Shiro uses for the same attribute.
    pub const IDENTITY_REMOVED_KEY: &'static str = "shiroIdentityRemoved";

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
        if let HttpMethod::GET = req.method() {
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
        req.parameter(name).map(|s| s.trim())
    }

    pub fn is_true(req: &dyn HttpRequest, param_name: &str) -> bool {
        Self::get_clean_param(req, param_name)
            .map(Self::is_true_value)
            .unwrap_or_default()
    }

    /// Returns whether a parameter value is one of the spellings of `true`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value of the parameter.
    pub fn is_true_value(value: &str) -> bool {
        !value.is_empty()
            && (value.eq_ignore_ascii_case("true")
                || value.eq_ignore_ascii_case("t")
                || value.eq_ignore_ascii_case("1")
                || value.eq_ignore_ascii_case("enabled")
                || value.eq_ignore_ascii_case("y")
                || value.eq_ignore_ascii_case("yes")
                || value.eq_ignore_ascii_case("on"))
    }

    /// The attribute the filter chain binds the subject of the call under.
    pub const SHARED_SUBJECT_KEY: &'static str = "NextWebSubject";

    pub async fn get_subject(
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Box<dyn Subject> {
        // The subject the filter chain is serving is the one the call was given,
        // which the proxy binds to the request before the chain runs.
        if let Some(next_web_core::any::AnyValue::BoxedValue(object)) =
            req.get_attribute(Self::SHARED_SUBJECT_KEY)
        {
            if let Some(subject) = (object.as_ref() as &dyn Any)
                .downcast_ref::<Arc<dyn crate::web::subject::web_subject::WebSubject>>()
            {
                let cloned: Box<dyn crate::web::subject::web_subject::WebSubject> =
                    next_web_core::clone_box(&**subject);

                return cloned;
            }
        }

        if let Some(subject) = req.get_attribute("NextSubject") {
            match subject {
                AnyValue::BoxedValue(obj) => {
                    if let Some(subject) = (obj as &dyn Any).downcast_ref::<WebDelegatingSubject>()
                    {
                        return Box::new(subject.clone());
                    }
                }
                _ => {}
            }
        }

        req.get_attribute(DEFAULT_WEB_DELEGATING_SUBJECT_KEY)
            .unwrap()
            .to_value::<Arc<dyn SecurityManager>>()
            .unwrap()
            .create_subject(Arc::new(DefaultWebSubjectContext::default()), req, resp)
            .await
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};

    use super::WebUtils;

    fn request(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("the request is built")
    }

    #[test]
    fn the_url_of_a_request_is_its_path_and_its_query() {
        let mut request = request("/orders/42?page=2&size=10");

        assert_eq!(
            WebUtils::get_request_url(&mut request),
            "/orders/42?page=2&size=10"
        );
    }

    #[test]
    fn a_parameter_is_read_as_the_request_carries_it() {
        // The value of a parameter is not percent-decoded by the request, which is
        // what Shiro's `getCleanParam` returns as well: it trims the value and
        // leaves the decoding to the caller that needs it.
        let request = request("/login?username=root");

        assert_eq!(
            WebUtils::get_clean_param(&request, "username"),
            Some("root")
        );
        assert_eq!(WebUtils::get_clean_param(&request, "missing"), None);
    }

    #[test]
    fn a_parameter_is_true_in_the_spellings_a_browser_sends() {
        for value in ["true", "TRUE", "t", "1", "enabled", "y", "yes", "on"] {
            let request = request(&format!("/x?flag={value}"));
            assert!(WebUtils::is_true(&request, "flag"), "value {value} is true");
        }

        for value in ["false", "0", "no", ""] {
            let request = request(&format!("/x?flag={value}"));
            assert!(
                !WebUtils::is_true(&request, "flag"),
                "value {value} is not true"
            );
        }

        let request = request("/x");
        assert!(!WebUtils::is_true(&request, "flag"));
    }
}
