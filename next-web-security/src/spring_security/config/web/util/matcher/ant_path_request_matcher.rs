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
        if let Some(http_method) = self.http_method {
            if request.method().as_str() != http_method.to_string() {
                return false;
            }
        }

        let path = request.uri().path();
        if self.pattern == Self::MATCH_ALL {
            return true;
        }

        if self.case_sensitive {
            ant_match(&self.pattern, path)
        } else {
            ant_match(&self.pattern.to_ascii_lowercase(), &path.to_ascii_lowercase())
        }
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

fn ant_match(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }

    let pattern_segments = split_path(pattern);
    let path_segments = split_path(path);
    match_segments(&pattern_segments, &path_segments)
}

fn split_path(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    if pattern[0] == "**" {
        return match_segments(&pattern[1..], path)
            || (!path.is_empty() && match_segments(pattern, &path[1..]));
    }

    if path.is_empty() {
        return false;
    }

    match_segment(pattern[0], path[0]) && match_segments(&pattern[1..], &path[1..])
}

fn match_segment(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let (mut p, mut t) = (0, 0);
    let mut star = None;
    let mut star_match = 0;

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            star_match = t;
            p += 1;
        } else if let Some(star_index) = star {
            p = star_index + 1;
            star_match += 1;
            t = star_match;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }

    p == pattern.len()
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
