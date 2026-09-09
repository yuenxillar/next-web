use std::fmt;
use std::sync::Arc;
use std::{collections::BTreeMap, sync::OnceLock};

use crate::web::util::matcher::{AnyRequestMatcher, MatchResult, RequestMatcher};
use next_web_core::http::HttpMethod;
use next_web_core::{traits::http::http_request::HttpRequest, util::pattern::PathPatternParser};

/// A request matcher that uses path patterns to match against each request.
///
/// The provided path should be relative to the context path (that is, it should
/// exclude any context path).
///
/// # Pattern syntax
///
/// Path patterns always start with a slash and may contain placeholders. They can
/// also be followed by `/**` to signify all URIs under a given path.
///
/// The following are valid patterns and their meaning:
/// * `/path` - match exactly and only `/path`
/// * `/path/**` - match `/path` and any of its descendants
/// * `/path/{value}/**` - match `/path/subdirectory` and any of its descendants,
///   capturing the value of the subdirectory
///
/// # Examples
///
/// ```
/// use PathPatternRequestMatcher;
///
/// let matcher = PathPatternRequestMatcher::path_pattern(None, "/api/users/**");
/// ```
#[derive(Debug, Clone)]
pub struct PathPatternRequestMatcher {
    pattern: Arc<PathPattern>,
    method: Arc<dyn RequestMatcher>,
    method_key: Option<HttpMethod>,
}

impl PathPatternRequestMatcher {
    /// Creates a new `PathPatternRequestMatcher` with the given pattern and method matcher.
    fn new(
        pattern: Arc<PathPattern>,
        method: Arc<dyn RequestMatcher>,
        method_key: Option<HttpMethod>,
    ) -> Self {
        Self {
            pattern,
            method,
            method_key,
        }
    }

    /// Constructs a `PathPatternRequestMatcher` using default settings.
    ///
    /// The pattern should be relative to the context path.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to match. `None` indicates the method does not matter.
    /// * `pattern` - The URI pattern to match.
    ///
    /// # Returns
    ///
    /// A `PathPatternRequestMatcher` that matches requests to the given pattern and method.
    ///
    /// # Examples
    ///
    /// ```
    /// let matcher = PathPatternRequestMatcher::path_pattern(None, "/api/**");
    /// let matcher = PathPatternRequestMatcher::path_pattern(
    ///     Some(HttpMethod::GET),
    ///     "/users/{id}"
    /// );
    /// ```
    pub fn path_pattern(method: Option<HttpMethod>, pattern: &str) -> Self {
        Self::with_defaults().matcher(method, pattern)
    }

    /// Creates a new `Builder` with default settings.
    ///
    /// # Returns
    ///
    /// A `Builder` that treats URIs as relative to the context path, if any.
    pub fn with_defaults() -> Builder {
        Builder::default()
    }

    pub fn with_path_pattern_parser(parser: PathPatternParser) -> Builder {
        Builder::with_parser(Arc::new(parser))
    }
}

impl PartialEq for PathPatternRequestMatcher {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern && self.method_key == other.method_key
    }
}

impl Eq for PathPatternRequestMatcher {}

impl std::hash::Hash for PathPatternRequestMatcher {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pattern.hash(state);
        self.method_key.hash(state);
    }
}

impl fmt::Display for PathPatternRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.method_key.as_ref() {
            Some(method) => write!(f, "PathPattern [{} {}]", method, self.pattern),
            None => write!(f, "PathPattern [{}]", self.pattern),
        }
    }
}

impl RequestMatcher for PathPatternRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        self.matcher(request).is_match()
    }

    fn matcher(&self, request: &dyn HttpRequest) -> MatchResult {
        if !self.method.matches(request) {
            return MatchResult::not_match();
        }

        match self.pattern.match_and_extract(request.path()) {
            Some(vars) => MatchResult::match_with_variables(vars),
            None => MatchResult::not_match(),
        }
    }
}

/// A compiled path pattern that can extract URI variables.
#[derive(Debug, Clone)]
struct PathPattern {
    segments: Vec<String>,
    pattern_string: String,
    case_sensitive: bool,
}

impl PathPattern {
    /// Parses a path pattern string into a compiled `PathPattern`.
    ///
    /// # Panics
    ///
    /// Panics if the pattern is malformed or contains invalid regex syntax.
    fn parse(pattern: &str, case_sensitive: bool) -> Self {
        assert!(pattern.starts_with('/'), "pattern must start with a /");
        let segments = pattern.split('/').skip(1).map(str::to_string).collect();
        Self {
            segments,
            pattern_string: pattern.to_string(),
            case_sensitive,
        }
    }

    /// Converts a path pattern string to a regular expression.
    ///
    /// Handles `{variable}` placeholders and `**` wildcards.
    fn match_and_extract(&self, path: &str) -> Option<BTreeMap<String, String>> {
        let values: Vec<&str> = path.split('/').skip(1).collect();
        let mut vars = BTreeMap::new();
        let mut pi = 0;
        for (i, segment) in self.segments.iter().enumerate() {
            if segment == "**" && i + 1 == self.segments.len() {
                return Some(vars);
            }
            if segment.starts_with("{*") && segment.ends_with('}') && i + 1 == self.segments.len() {
                vars.insert(
                    segment[2..segment.len() - 1].to_string(),
                    values[pi..].join("/"),
                );
                return Some(vars);
            }
            let raw_value = *values.get(pi)?;
            let lowered;
            let value = if self.case_sensitive {
                raw_value
            } else {
                lowered = raw_value.to_ascii_lowercase();
                &lowered
            };
            if segment.starts_with('{') && segment.ends_with('}') {
                let body = &segment[1..segment.len() - 1];
                let (name, expr) = body.split_once(':').unwrap_or((body, ""));
                if !expr.is_empty() && !simple_regex_match(expr, value) {
                    return None;
                }
                vars.insert(name.to_string(), value.to_string());
            } else {
                let lowered_segment;
                let segment = if self.case_sensitive {
                    segment.as_str()
                } else {
                    lowered_segment = segment.to_ascii_lowercase();
                    &lowered_segment
                };
                if !segment_match(segment, value) {
                    return None;
                }
            }
            pi += 1;
        }
        (pi == values.len()).then_some(vars)
    }
}

impl PartialEq for PathPattern {
    fn eq(&self, other: &Self) -> bool {
        self.pattern_string == other.pattern_string
    }
}

impl Eq for PathPattern {}

impl std::hash::Hash for PathPattern {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pattern_string.hash(state);
    }
}

impl fmt::Display for PathPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pattern_string)
    }
}

fn segment_match(pattern: &str, value: &str) -> bool {
    let (p, v): (Vec<char>, Vec<char>) = (pattern.chars().collect(), value.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, None, 0);
    while j < v.len() {
        if i < p.len() && (p[i] == '?' || p[i] == v[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some(i);
            i += 1;
            mark = j;
        } else if let Some(s) = star {
            i = s + 1;
            mark += 1;
            j = mark;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

fn simple_regex_match(expr: &str, value: &str) -> bool {
    // Covers the expressions commonly used by Spring path variables.
    if expr == ".*" {
        return true;
    }
    if expr == "\\w+" {
        return value.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) && !value.is_empty();
    }
    if let Some(class) = expr
        .strip_suffix('+')
        .and_then(|s| s.strip_prefix('[').and_then(|s| s.strip_suffix(']')))
    {
        let valid = |c: char| match class {
            "a-z" => c.is_ascii_lowercase(),
            "A-Z" => c.is_ascii_uppercase(),
            "0-9" => c.is_ascii_digit(),
            _ => class.contains(c),
        };
        return !value.is_empty() && value.chars().all(valid);
    }
    segment_match(expr, value)
}

pub static DEFAULT_BUILDER: OnceLock<Builder> = OnceLock::new();

/// A builder for specifying various elements of a request for the purpose
/// of creating a `PathPatternRequestMatcher`.
///
/// # Examples
///
/// To match a request URI like `/app/servlet/my/resource/**` where `/app`
/// is the context path, you can do:
///
/// ```
/// let matcher = PathPatternRequestMatcher::path_pattern(
///     None,
///     "/servlet/my/resource/**"
/// );
/// ```
///
/// If you have many paths that have a common path prefix, you can use
/// `base_path` to reduce repetition like so:
///
/// ```
/// let mvc = PathPatternRequestMatcher::with_defaults()
///     .base_path("/mvc");
///
/// let user_matcher = mvc.matcher(None, "/user/**");
/// let admin_matcher = mvc.matcher(None, "/admin/**");
/// ```
#[derive(Debug, Clone)]
pub struct Builder {
    parser: Arc<PathPatternParser>,
    base_path: String,
}

impl Builder {
    /// Creates a new `Builder` with an empty base path.
    #[allow(unused)]
    fn new(parser: Arc<PathPatternParser>, base_path: impl Into<String>) -> Self {
        Self {
            parser,
            base_path: base_path.into(),
        }
    }

    fn with_parser(parser: Arc<PathPatternParser>) -> Self {
        Self {
            parser,
            base_path: String::new(),
        }
    }

    /// Match requests starting with this `base_path`.
    ///
    /// Prefixes should be of the form `/my/prefix`, starting with a slash,
    /// not ending in a slash, and not containing any wildcards. The special
    /// value `"/"` may be used to indicate the root context.
    ///
    /// # Arguments
    ///
    /// * `base_path` - The path prefix.
    ///
    /// # Returns
    ///
    /// The `Builder` for further configuration.
    ///
    /// # Panics
    ///
    /// Panics if `base_path` is empty, does not start with '/', ends with '/'
    /// (unless it is "/"), or contains a '*'.
    pub fn base_path(mut self, base_path: &str) -> Self {
        assert!(!base_path.is_empty(), "base_path cannot be empty");
        assert!(base_path.starts_with('/'), "base_path must start with '/'");
        assert!(
            base_path == "/" || !base_path.ends_with('/'),
            "base_path must not end with a slash"
        );
        assert!(
            !base_path.contains('*'),
            "base_path must not contain a star"
        );

        self.base_path = base_path.to_string();
        self
    }

    /// Match requests having this path pattern.
    ///
    /// When the HTTP method is `None`, then the matcher does not consider
    /// the HTTP method.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to match, or `None` to match any method.
    /// * `path` - The path pattern to match.
    ///
    /// # Returns
    ///
    /// A `PathPatternRequestMatcher` configured with the given parameters.
    ///
    /// # Panics
    ///
    /// Panics if `path` is empty or does not start with '/'.
    pub fn matcher(&self, method: Option<HttpMethod>, path: &str) -> PathPatternRequestMatcher {
        assert!(!path.is_empty(), "pattern cannot be empty");
        assert!(path.starts_with('/'), "pattern must start with a /");

        let prefix = if self.base_path == "/" {
            ""
        } else {
            self.base_path.as_str()
        };

        let full_pattern = format!("{}{}", prefix, path);
        let path_pattern = Arc::new(PathPattern::parse(
            &full_pattern,
            self.parser.is_case_sensitive(),
        ));

        let method_matcher = match method.clone() {
            Some(method) => Arc::new(HttpMethodRequestMatcher::new(method.clone())),
            None => AnyRequestMatcher::instance(),
        };

        PathPatternRequestMatcher::new(path_pattern, method_matcher, method)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            parser: PathPatternParser::default_instance(),
            base_path: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct HttpMethodRequestMatcher(HttpMethod);

impl HttpMethodRequestMatcher {
    pub fn new(method: HttpMethod) -> Self {
        Self(method)
    }
}

impl RequestMatcher for HttpMethodRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        request.method() == self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_variables() {
        let pattern = PathPattern::parse("/users/{id}/posts/{post_id}", true);

        let result = pattern.match_and_extract("/users/123/posts/456");
        assert!(result.is_some());

        let vars = result.unwrap();
        assert_eq!(vars.get("id").unwrap(), "123");
        assert_eq!(vars.get("post_id").unwrap(), "456");
    }

    #[test]
    fn test_wildcard_pattern() {
        let pattern = PathPattern::parse("/api/**", true);

        assert!(pattern.match_and_extract("/api/users").is_some());
        assert!(pattern.match_and_extract("/api/users/123").is_some());
        assert!(pattern.match_and_extract("/other").is_none());
    }

    #[test]
    fn test_base_path_builder() {
        let builder = Builder::default().base_path("/mvc");

        let matcher = builder.matcher(None, "/user/**");

        // Verify the pattern includes the base path
        assert!(matcher.pattern.pattern_string == "/mvc/user/**");
    }
}
