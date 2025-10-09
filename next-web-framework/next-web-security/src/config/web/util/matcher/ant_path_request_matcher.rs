use next_web_core::util::http_method::HttpMethod;

use crate::config::web::util::matcher::request_matcher::RequestMatcher;

#[derive(Clone, Debug)]
pub struct AntPathRequestMatcher {
    pattern: String,
    http_method: Option<HttpMethod>,
    case_sensitive: bool,
}

impl AntPathRequestMatcher {
    const MATCH_ALL: &str = "/**";

    pub fn new(pattern: impl ToString) -> Self {
        Self {
            pattern: pattern.to_string(),
            http_method: None,
            case_sensitive: true,
        }
    }
}

impl RequestMatcher for AntPathRequestMatcher {
    fn matches(&self, request: &axum::extract::Request) -> bool {
        todo!()
    }
}

impl From<HttpMethod> for AntPathRequestMatcher {
    fn from(method: HttpMethod) -> Self {
        Self {
            pattern: String::from("/**"),
            http_method: Some(method),
            case_sensitive: true,
        }
    }
}

impl From<&str> for AntPathRequestMatcher {
    fn from(pattern: &str) -> Self {
        assert!(!pattern.is_empty(), "pattern cannot be empty");
        Self {
            pattern: pattern.to_string(),
            http_method: None,
            case_sensitive: true,
        }
    }
}

impl From<(HttpMethod, &str)> for AntPathRequestMatcher {
    fn from((method, pattern): (HttpMethod, &str)) -> Self {
        assert!(!pattern.is_empty(), "pattern cannot be empty");
        Self {
            pattern: pattern.to_string(),
            http_method: Some(method),
            case_sensitive: true,
        }
    }
}


impl From<(Option<HttpMethod>, &str)> for AntPathRequestMatcher {
    fn from((method, pattern): (Option<HttpMethod>, &str)) -> Self {
        assert!(!pattern.is_empty(), "pattern cannot be empty");
        Self {
            pattern: pattern.to_string(),
            http_method: method,
            case_sensitive: true,
        }
    }
}