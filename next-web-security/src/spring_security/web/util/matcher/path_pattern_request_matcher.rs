use std::fmt;
use std::sync::Arc;
use std::{collections::HashMap, sync::OnceLock};

use next_web_core::{traits::http::http_request::HttpRequest, util::pattern::PathPatternParser};
use regex::Regex;

use crate::web::util::matcher::{AnyRequestMatcher, MatchResult, RequestMatcher};
use next_web_core::http::HttpMethod;

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
}

impl PathPatternRequestMatcher {
    /// Creates a new `PathPatternRequestMatcher` with the given pattern and method matcher.
    fn new(pattern: Arc<PathPattern>, method: Arc<dyn RequestMatcher>) -> Self {
        Self { pattern, method }
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
        Arc::ptr_eq(&self.pattern, &other.pattern) || self.pattern == other.pattern
        // Note: method_matcher comparison is simplified here; in production,
        // you'd implement PartialEq for HttpMethodMatcher as well
    }
}

impl Eq for PathPatternRequestMatcher {}

impl std::hash::Hash for PathPatternRequestMatcher {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pattern.hash(state);
        // Hash method_matcher appropriately in production
    }
}

impl fmt::Display for PathPatternRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PathPattern [{:?}{}]", self.method, self.pattern)
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

        MatchResult::not_match()
    }
}

/// A compiled path pattern that can extract URI variables.
#[derive(Debug, Clone)]
struct PathPattern {
    regex: Regex,
    variable_names: Vec<String>,
    pattern_string: String,
}

impl PathPattern {
    /// Parses a path pattern string into a compiled `PathPattern`.
    ///
    /// # Panics
    ///
    /// Panics if the pattern is malformed or contains invalid regex syntax.
    fn parse(pattern: &str) -> Self {
        let mut variable_names = Vec::new();
        let regex_str = Self::pattern_to_regex(pattern, &mut variable_names);

        let regex = Regex::new(&format!("^{}$", regex_str))
            .unwrap_or_else(|e| panic!("Invalid path pattern '{}': {}", pattern, e));

        Self {
            regex,
            variable_names,
            pattern_string: pattern.to_string(),
        }
    }

    /// Converts a path pattern string to a regular expression.
    ///
    /// Handles `{variable}` placeholders and `**` wildcards.
    fn pattern_to_regex(pattern: &str, variable_names: &mut Vec<String>) -> String {
        // Remove leading slash for regex construction
        let pattern = pattern.trim_start_matches('/');

        let mut regex = String::from("");

        for segment in pattern.split('/') {
            if !regex.is_empty() {
                regex.push('/');
            }

            if segment == "**" {
                regex.push_str(".*");
            } else if segment.starts_with('{') && segment.ends_with('}') {
                let var_name = &segment[1..segment.len() - 1];
                variable_names.push(var_name.to_string());
                regex.push_str("([^/]+)");
            } else {
                regex.push_str(&regex::escape(segment));
            }
        }

        format!("/{}", regex)
    }

    /// Attempts to match a path against this pattern and extract variables.
    fn match_and_extract(&self, path: &str) -> Option<HashMap<String, String>> {
        self.regex.captures(path).map(|captures| {
            let mut variables = HashMap::new();
            for (i, name) in self.variable_names.iter().enumerate() {
                if let Some(value) = captures.get(i + 1) {
                    variables.insert(name.clone(), value.as_str().to_string());
                }
            }
            variables
        })
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
    fn new(parser: Arc<PathPatternParser>, base_path: String) -> Self {
        Self { parser, base_path }
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
            String::new()
        } else {
            self.base_path.clone()
        };

        let full_pattern = format!("{}{}", prefix, path);
        let path_pattern = Arc::new(PathPattern::parse(&full_pattern));

        let method_matcher = match method {
            Some(method) => Arc::new(HttpMethodRequestMatcher::new(method)),
            None => AnyRequestMatcher::instance(),
        };

        PathPatternRequestMatcher::new(path_pattern, method_matcher)
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
        let pattern = PathPattern::parse("/users/{id}/posts/{post_id}");

        let result = pattern.match_and_extract("/users/123/posts/456");
        assert!(result.is_some());

        let vars = result.unwrap();
        assert_eq!(vars.get("id").unwrap(), "123");
        assert_eq!(vars.get("post_id").unwrap(), "456");
    }

    #[test]
    fn test_wildcard_pattern() {
        let pattern = PathPattern::parse("/api/**");

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
