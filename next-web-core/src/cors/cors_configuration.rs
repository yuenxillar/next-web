use std::{
    collections::HashSet,
    error::Error,
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
    sync::LazyLock,
    time::Duration,
};

use regex::Regex;

use crate::{http::HttpMethod, util::StringUtils};

/// Errors that can occur during CORS configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsError {
    InvalidHttpMethod(String),
    AllowCredentialsWithWildcard,
    AllowPrivateNetworkWithWildcard,
    InvalidOriginPattern(String),
}

impl Error for CorsError {}

impl Display for CorsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorsError::InvalidHttpMethod(method) => write!(f, "Invalid HTTP method: {}", method),
            CorsError::AllowCredentialsWithWildcard => write!(
                f,
                "When allow_credentials is true, allowed_origins cannot contain the special value \"*\""
            ),
            CorsError::AllowPrivateNetworkWithWildcard => write!(
                f,
                "When allow_private_network is true, allowed_origins cannot contain the special value \"*\""
            ),
            CorsError::InvalidOriginPattern(pattern) => {
                write!(f, "Invalid origin pattern: {}", pattern)
            }
        }
    }
}

/// Result type for CORS operations.
pub type CorsResult<T> = Result<T, CorsError>;

const ALL: &str = "*";
static DEFAULT_PERMIT_ALL: &[&str] = &[ALL];
static DEFAULT_METHODS: &[HttpMethod] = &[HttpMethod::GET, HttpMethod::HEAD];
static DEFAULT_PERMIT_METHODS: &[&str] = &["GET", "HEAD", "POST"];
static ALL_PATTERN: LazyLock<OriginPattern> = LazyLock::new(|| match OriginPattern::new(ALL) {
    Ok(pattern) => pattern,
    Err(err) => panic!("{}", err),
});
static ALL_PATTERN_LIST: LazyLock<Vec<OriginPattern>> = LazyLock::new(|| vec![ALL_PATTERN.clone()]);

/// Configuration for Cross-Origin Resource Sharing (CORS).
///
/// By default, a newly created `CorsConfiguration` does not permit any
/// cross-origin requests and must be configured explicitly to indicate what
/// should be allowed. Use `apply_permit_default_values()` to start with open
/// defaults that permit all cross-origin requests for GET, HEAD, and POST requests.
#[derive(Clone, Debug, Default)]
pub struct CorsConfiguration {
    allowed_origins: Option<Vec<String>>,
    allowed_origin_patterns: Option<Vec<OriginPattern>>,
    allowed_methods: Option<Vec<String>>,
    resolved_methods: Option<Vec<HttpMethod>>,
    allowed_headers: Option<Vec<String>>,
    exposed_headers: Option<Vec<String>>,
    allow_credentials: Option<bool>,
    allow_private_network: Option<bool>,
    max_age: Option<u64>,
}

impl CorsConfiguration {
    /// Creates a new configuration by copying all values from another.
    pub fn from_other(other: &CorsConfiguration) -> Self {
        Self {
            allowed_origins: other.allowed_origins.clone(),
            allowed_origin_patterns: other.allowed_origin_patterns.clone(),
            allowed_methods: other.allowed_methods.clone(),
            resolved_methods: other.resolved_methods.clone(),
            allowed_headers: other.allowed_headers.clone(),
            exposed_headers: other.exposed_headers.clone(),
            allow_credentials: other.allow_credentials,
            allow_private_network: other.allow_private_network,
            max_age: other.max_age,
        }
    }

    /// Returns a list of origins for which cross-origin requests are allowed.
    pub fn allowed_origins(&self) -> Option<&[String]> {
        self.allowed_origins.as_deref()
    }

    /// Sets the list of allowed origins.
    ///
    /// Each value may be:
    /// - A specific domain, e.g., "https://domain1.com"
    /// - Comma-delimited list of specific domains
    /// - The special value "*" for all origins
    pub fn set_allowed_origins(&mut self, origins: Option<Vec<String>>) -> CorsResult<()> {
        match origins {
            None => self.allowed_origins = None,
            Some(origins) => {
                self.allowed_origins = Some(Vec::with_capacity(origins.len()));
                for origin in origins {
                    self.add_allowed_origin(&origin)?;
                }
            }
        };
        Ok(())
    }

    /// Adds a single allowed origin.
    pub fn add_allowed_origin(&mut self, origin: &str) -> CorsResult<()> {
        if origin.is_empty() {
            return Ok(());
        }

        match self.allowed_origins.as_ref() {
            Some(allowed_origins) => {
                if allowed_origins == DEFAULT_PERMIT_ALL
                    && self
                        .allowed_origin_patterns
                        .as_ref()
                        .map(Vec::is_empty)
                        .unwrap_or(true)
                {
                    self.set_allowed_origins(Some(
                        DEFAULT_PERMIT_ALL
                            .into_iter()
                            .map(ToString::to_string)
                            .collect(),
                    ))?;
                }
            }
            None => {
                self.allowed_origins = Some(Vec::with_capacity(4));
            }
        }

        Self::parse_comma_delimited_origin(origin, |value| {
            let trimmed = Self::trim_trailing_slash(value);
            self.allowed_origins
                .as_mut()
                .map(|ao| ao.push(trimmed.to_string()));
        });
        Ok(())
    }

    /// Returns the configured origin patterns to allow, or None if none.
    pub fn allowed_origin_patterns(&self) -> Option<Vec<String>> {
        self.allowed_origin_patterns.as_ref().map(|patterns| {
            patterns
                .iter()
                .map(|p| p.declared_pattern().to_string())
                .collect()
        })
    }

    /// Sets the list of allowed origin patterns.
    ///
    /// Supports flexible origin patterns with "*" anywhere in the host name
    /// in addition to port lists.
    pub fn set_allowed_origin_patterns(
        &mut self,
        allowed_origin_patterns: Option<Vec<String>>,
    ) -> &mut Self {
        match allowed_origin_patterns {
            None => self.allowed_origin_patterns = None,
            Some(allowed_origin_patterns) => {
                self.allowed_origin_patterns =
                    Some(Vec::with_capacity(allowed_origin_patterns.len()));
                for pattern_value in allowed_origin_patterns {
                    self.add_allowed_origin_pattern(&pattern_value);
                }
            }
        }
        self
    }

    /// Adds a single allowed origin pattern.
    pub fn add_allowed_origin_pattern(&mut self, origin_pattern: &str) {
        if origin_pattern.is_empty() {
            return;
        }

        let origin_patterns = self
            .allowed_origin_patterns
            .get_or_insert_with(|| Vec::with_capacity(4));

        Self::parse_comma_delimited_origin(origin_pattern, |value| {
            let trimmed = Self::trim_trailing_slash(value);
            match OriginPattern::new(trimmed) {
                Ok(pattern) => origin_patterns.push(pattern),
                Err(err) => panic!("{}", err),
            };
            if let Some(allowed_origins) = &self.allowed_origins {
                if allowed_origins == DEFAULT_PERMIT_ALL {
                    self.allowed_origins = None;
                }
            }
        });
    }

    /// Parses a comma-delimited origin string, respecting port ranges.
    ///
    /// Port ranges in square brackets (e.g., `:[8080,9090]`) are preserved
    /// and commas inside them are not treated as delimiters.
    fn parse_comma_delimited_origin<F>(raw_value: &str, mut value_consumer: F)
    where
        F: FnMut(&str),
    {
        if !raw_value.contains(',') {
            let trimmed = raw_value.trim();
            if !trimmed.is_empty() {
                value_consumer(trimmed);
            }
            return;
        }

        let mut start = 0;
        let mut within_port_range = false;

        for (current, ch) in raw_value.char_indices() {
            match ch {
                '[' => within_port_range = true,
                ']' => within_port_range = false,
                ',' => {
                    if !within_port_range {
                        let origin_value = raw_value[start..current].trim();
                        if !origin_value.is_empty() {
                            value_consumer(origin_value);
                        }
                        start = current + 1;
                    }
                }
                _ => {}
            }
        }

        if start < raw_value.len() {
            let origin_value = raw_value[start..].trim();
            if !origin_value.is_empty() {
                value_consumer(origin_value);
            }
        }
    }

    /// Return the allowed HTTP methods, or null in which case only "GET" and "HEAD" allowed.
    pub fn allowed_methods(&self) -> Option<&[String]> {
        self.allowed_methods.as_deref()
    }

    /// Set the HTTP methods to allow, for example, "GET", "POST", "PUT", etc. The special value "*" allows all methods.
    ///
    /// Access-Control-Allow-Methods response header is set either to the configured method or to
    /// "*". Keep in mind however that the CORS spec does not allow "*" when allowCredentials is
    /// set to true, that combination is handled by copying the method specified in the CORS preflight request.
    ///
    /// If not set, only "GET" and "HEAD" are allowed.
    pub fn set_allowed_methods(&mut self, methods: Option<Vec<String>>) -> CorsResult<()> {
        self.allowed_methods = methods.clone();
        self.resolved_methods = match methods {
            None => Some(DEFAULT_METHODS.to_vec()),
            Some(allowed_methods) => {
                let mut resolved = Vec::with_capacity(allowed_methods.len());
                for method in &allowed_methods {
                    if method == ALL {
                        self.resolved_methods = None;
                        return Ok(());
                    }
                    resolved.push(
                        HttpMethod::from_str(method)
                            .map_err(|err| CorsError::InvalidHttpMethod(err.to_string()))?,
                    );
                }
                Some(resolved)
            }
        };
        Ok(())
    }

    /// Variant of setAllowedMethods for adding one allowed method at a time.
    pub fn add_allowed_method(&mut self, method: HttpMethod) -> CorsResult<()> {
        self.add_allowed_method_with_str(method.as_str())
    }

    /// Variant of setAllowedMethods for adding one allowed method at a time.
    pub fn add_allowed_method_with_str(&mut self, method: &str) -> CorsResult<()> {
        if !StringUtils::has_text(method) {
            return Ok(());
        }

        match self.allowed_methods.as_ref() {
            Some(allowed_methods) => {
                if allowed_methods == DEFAULT_PERMIT_METHODS {
                    self.set_allowed_methods(Some(
                        DEFAULT_PERMIT_METHODS
                            .into_iter()
                            .map(ToString::to_string)
                            .collect(),
                    ))?;
                }
            }
            None => {
                self.allowed_methods = Some(Vec::with_capacity(4));
                self.resolved_methods = Some(Vec::with_capacity(4));
            }
        }
        self.allowed_methods
            .as_mut()
            .map(|am| am.push(method.to_string()));

        if method == ALL {
            self.resolved_methods = None;
        } else {
            let method = HttpMethod::from_str(method)
                .map_err(|err| CorsError::InvalidHttpMethod(err.to_string()))?;
            self.resolved_methods.as_mut().map(|rm| {
                rm.push(method);
            });
        }
        Ok(())
    }

    /// Returns the allowed request headers.
    pub fn allowed_headers(&self) -> Option<&[String]> {
        self.allowed_headers.as_deref()
    }

    /// Set the list of headers that a pre-flight request can list as allowed for use during an actual request. The special value "*" allows actual requests to send any header.
    /// Access-Control-Allow-Headers response header is set either to the configured list of headers or to "*". Keep in mind however that the CORS spec does not allow
    /// "*" when allowCredentials is set to true, that combination is handled by copying the headers specified in the CORS preflight request.
    ///
    /// A header name is not required to be listed if it is one of: Cache-Control, Content-Language, Expires, Last-Modified, or Pragma.
    pub fn set_allowed_headers(&mut self, headers: Option<Vec<String>>) {
        self.allowed_headers = headers.map(|h| h.into_iter().filter(|s| !s.is_empty()).collect());
    }

    /// Return the allowed actual request headers
    pub fn allowed_header(&self) -> Option<&[String]> {
        self.allowed_headers.as_deref()
    }

    /// Variant of set_allowed_headers(List) for adding one allowed header at a time.
    pub fn add_allowed_header(&mut self, allowed_header: &str) {
        match self.allowed_headers.as_ref() {
            Some(allowed_headers) => {
                if allowed_headers == DEFAULT_PERMIT_ALL {
                    self.set_allowed_headers(Some(
                        DEFAULT_PERMIT_ALL.iter().map(ToString::to_string).collect(),
                    ));
                }
            }
            None => {
                self.allowed_headers = Some(Vec::with_capacity(4));
            }
        }
        self.allowed_headers
            .as_mut()
            .map(|ah| ah.push(allowed_header.to_string()));
    }

    /// Returns the exposed response headers.
    pub fn exposed_headers(&self) -> Option<&[String]> {
        self.exposed_headers.as_deref()
    }

    /// Set the list of response headers that an actual response might have and can be exposed to the client. The special value "*" allows all headers to be exposed.
    /// Access-Control-Expose-Headers response header is set either to the configured list of headers or to "*". While the CORS spec does not allow "*"
    /// when Access-Control-Allow-Credentials is set to true, most browsers support it and the response headers are not all available during the CORS processing,
    /// so as a consequence "*" is the header value used when specified regardless of the value of the `allowCredentials` property.
    ///
    /// A header name is not required to be listed if it is one of: Cache-Control, Content-Language, Expires, Last-Modified, or Pragma.
    ///
    /// By default this is not set.
    pub fn set_exposed_headers(&mut self, headers: Option<Vec<String>>) {
        self.exposed_headers = headers.map(|h| h.into_iter().filter(|s| !s.is_empty()).collect());
    }

    /// Variant of set_exposed_headers for adding one exposed header at a time.
    pub fn add_exposed_header(&mut self, exposed_header: &str) {
        let exposed_headers = self
            .exposed_headers
            .get_or_insert_with(|| Vec::with_capacity(4));
        exposed_headers.push(exposed_header.to_string());
    }

    /// Returns whether user credentials are supported.
    pub fn allow_credentials(&self) -> Option<bool> {
        self.allow_credentials
    }

    /// Whether user credentials are supported.
    /// Setting this property has an impact on how origins, originPatterns, allowedMethods and allowedHeaders are processed, see related API documentation for more details.
    /// NOTE: Be aware that this option establishes a high level of trust with the configured domains and also increases the surface attack
    /// of the web application by exposing sensitive user-specific information such as cookies and CSRF tokens.
    ///
    /// By default, this is not set (i.e. user credentials are not supported).
    pub fn set_allow_credentials(&mut self, allow: Option<bool>) {
        self.allow_credentials = allow;
    }

    /// Returns whether private network access is supported.
    pub fn allow_private_network(&self) -> Option<bool> {
        self.allow_private_network
    }

    /// Whether private network access is supported for user-agents restricting such access by default.
    /// Private network requests are requests whose target server's IP address is more private than that from which the request initiator was fetched.
    /// For example, a request from a public website (https://example.com) to a private website (https://router.local), or a request from a private website to localhost.
    ///
    /// Setting this property has an impact on how origins and originPatterns are processed, see related API documentation for more details.
    ///
    /// By default, this is not set (i.e. private network access is not supported).
    pub fn set_allow_private_network(&mut self, allow: Option<bool>) {
        self.allow_private_network = allow;
    }

    /// Returns the max age for caching pre-flight responses.
    pub fn max_age(&self) -> Option<u64> {
        self.max_age
    }

    /// Configure how long, as a duration, the response from a pre-flight request can be cached by clients.
    pub fn set_max_age(&mut self, duration: Duration) {
        self.max_age = Some(duration.as_secs());
    }

    /// Sets the max age in seconds.
    pub fn set_max_age_with_secs(&mut self, seconds: Option<u64>) {
        self.max_age = seconds;
    }

    /// By default CorsConfiguration does not permit any cross-origin requests and must be configured
    /// explicitly. Use this method to switch to defaults that permit all cross-origin requests for
    /// GET, HEAD, and POST, but not overriding any values that have already been set.
    ///
    /// The following defaults are applied for values that are not set:
    /// - Allow all origins with the special value "*" defined in the CORS spec. This is set only if neither origins nor originPatterns are already set.
    /// - Allow "simple" methods GET, HEAD and POST.
    /// - Allow all headers.
    /// - Set max age to 1800 seconds (30 minutes).
    pub fn apply_permit_default_values(&mut self) -> &mut Self {
        if self.allowed_origins.is_none() && self.allowed_origin_patterns.is_none() {
            self.allowed_origins =
                Some(DEFAULT_PERMIT_ALL.iter().map(ToString::to_string).collect());
        }
        if self.allowed_methods.is_none() {
            let _methods = DEFAULT_PERMIT_METHODS
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            self.allowed_methods = Some(_methods.clone());
            self.resolved_methods = Some(
                _methods
                    .into_iter()
                    .filter_map(|var| HttpMethod::from_str(&var).ok())
                    .collect(),
            );
        }
        if self.allowed_headers.is_none() {
            self.allowed_headers =
                Some(DEFAULT_PERMIT_ALL.iter().map(ToString::to_string).collect());
        }
        if self.max_age.is_none() {
            self.max_age = Some(1800);
        }

        self
    }

    /// Validate that when allowCredentials is true, allowedOrigins does not contain the special
    /// value "*" since in that case the "Access-Control-Allow-Origin" cannot be set to "*".
    pub fn validate_allow_credentials(&self) -> CorsResult<()> {
        if let (Some(true), Some(origins)) = (self.allow_credentials, self.allowed_origins.as_ref())
        {
            if origins.iter().any(|o| o == ALL) {
                return Err(CorsError::AllowCredentialsWithWildcard);
            }
        }
        Ok(())
    }

    /// Validate that when allowPrivateNetwork is true, allowedOrigins does not contain the
    /// special value "*" since this is insecure.
    pub fn validate_allow_private_network(&self) -> CorsResult<()> {
        if let (Some(true), Some(origins)) = (self.allow_private_network, &self.allowed_origins) {
            if origins.iter().any(|o| o == ALL) {
                return Err(CorsError::AllowPrivateNetworkWithWildcard);
            }
        }
        Ok(())
    }

    /// Combine the non-null properties of the supplied CorsConfiguration with this one.
    /// When combining single values like allowCredentials or maxAge, this properties are overridden by non-null other properties if any.
    ///
    /// Combining lists like allowedOrigins, allowedMethods, allowedHeaders or exposedHeaders
    /// is done in an additive way. For example, combining ["GET", "POST"] with ["PATCH"] results in
    /// ["GET", "POST", "PATCH"]. However, combining ["GET", "POST"] with ["*"] results in ["*"].
    /// Note also that default permit values set by applyPermitDefaultValues() are
    /// overridden by any explicitly defined values.
    pub fn combine(&self, other: Option<&Self>) -> Self {
        let other = match other {
            Some(o) => o,
            None => return self.clone(),
        };

        // Bypass setAllowedOrigins to avoid re-compiling patterns
        let mut config = CorsConfiguration::from_other(self);
        let origins =
            Self::combine_string_lists(&self.allowed_origins, other.allowed_origins.as_ref());
        let patterns = Self::combine_pattern_lists(
            &self.allowed_origin_patterns,
            other.allowed_origin_patterns.as_ref(),
        );

        config.allowed_origins = {
            if origins
                .as_ref()
                .map(|var| var == DEFAULT_PERMIT_ALL)
                .unwrap_or_default()
                && patterns
                    .as_ref()
                    .map(|var| !var.is_empty())
                    .unwrap_or_default()
            {
                None
            } else {
                origins
            }
        };
        config.allowed_origin_patterns = patterns;

        // Combine methods
        config
            .set_allowed_methods(Self::combine_string_lists(
                &self.allowed_methods,
                other.allowed_methods.as_ref(),
            ))
            .map_err(|err| panic!("{}", err))
            .ok();

        // Combine headers
        config.set_allowed_headers(Self::combine_string_lists(
            &self.allowed_headers,
            other.allowed_headers.as_ref(),
        ));
        config.set_exposed_headers(Self::combine_string_lists(
            &self.exposed_headers,
            other.exposed_headers.as_ref(),
        ));

        // Override single values
        if let Some(allow_creds) = other.allow_credentials {
            config.allow_credentials = Some(allow_creds);
        }
        if let Some(allow_private) = other.allow_private_network {
            config.allow_private_network = Some(allow_private);
        }
        if let Some(max_age) = other.max_age {
            config.max_age = Some(max_age);
        }

        config
    }

    fn combine_string_lists(
        source: &Option<Vec<String>>,
        other: Option<&Vec<String>>,
    ) -> Option<Vec<String>> {
        let other = match other {
            Some(o) => o,
            None => return source.clone(),
        };

        let source = match source {
            Some(s) => s,
            None => return Some(other.clone()),
        };

        // If either is the special permit-all list, return the other
        if source == DEFAULT_PERMIT_ALL || source == DEFAULT_PERMIT_METHODS {
            return Some(other.clone());
        }

        if other == DEFAULT_PERMIT_ALL || other == DEFAULT_PERMIT_METHODS {
            return Some(source.clone());
        }

        if source.iter().any(|o| o == ALL) || other.iter().any(|o| o == ALL) {
            return Some(vec![ALL.to_string()]);
        }

        let mut combined = HashSet::with_capacity(source.len() + other.len());
        combined.extend(source.iter().cloned());
        combined.extend(other.iter().cloned());

        Some(combined.into_iter().collect())
    }

    fn combine_pattern_lists(
        source: &Option<Vec<OriginPattern>>,
        other: Option<&Vec<OriginPattern>>,
    ) -> Option<Vec<OriginPattern>> {
        let other = match other {
            Some(o) => o,
            None => return source.clone(),
        };

        let source = match source {
            Some(s) => s,
            None => return Some(other.clone()),
        };

        if source.contains(&ALL_PATTERN) || other.contains(&ALL_PATTERN) {
            return Some(ALL_PATTERN_LIST.clone());
        }

        // Check for wildcard patterns
        let wildcard = OriginPattern::new(ALL).ok()?;
        if source.contains(&wildcard) || other.contains(&wildcard) {
            return Some(vec![wildcard]);
        }

        let mut combined = HashSet::with_capacity(source.len() + other.len());
        combined.extend(source.iter().cloned());
        combined.extend(other.iter().cloned());

        Some(combined.into_iter().collect())
    }

    /// Checks if the request origin is allowed.
    ///
    /// Returns the origin to use for the response, or `None` if not allowed.
    pub fn check_origin(&self, origin: Option<&str>) -> CorsResult<Option<String>> {
        let origin = match origin.filter(|s| StringUtils::has_text(s)) {
            Some(o) => o,
            None => return Ok(None),
        };

        let origin_to_check = Self::trim_trailing_slash(origin);

        // Check exact origins
        if let Some(allowed_origins) = &self.allowed_origins {
            if allowed_origins.iter().any(|o| o == ALL) {
                self.validate_allow_credentials()?;
                self.validate_allow_private_network()?;
                return Ok(Some(ALL.to_string()));
            }
            for allowed in allowed_origins {
                if origin_to_check.eq_ignore_ascii_case(allowed) {
                    return Ok(Some(origin.to_string()));
                }
            }
        }

        // Check patterns
        if let Some(patterns) = &self.allowed_origin_patterns {
            for pattern in patterns {
                if pattern.declared_pattern() == ALL || pattern.matches(&origin_to_check) {
                    return Ok(Some(origin.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Check the HTTP request method (or the method from the Access-Control-Request-Method header on a pre-flight request)
    /// against the configured allowed methods.
    pub fn check_http_method(&self, method: Option<&HttpMethod>) -> Option<Vec<HttpMethod>> {
        let method = method?;

        match &self.resolved_methods {
            None => Some(vec![method.clone()]),
            Some(resolved) => {
                if resolved.contains(&method) {
                    Some(resolved.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Checks if the request headers are allowed.
    ///
    /// Returns the list of allowed headers to list in the response of a pre-flight request,
    /// or `None` if none of the supplied request headers is allowed.
    pub fn check_headers(&self, request_headers: Option<&[String]>) -> Option<Vec<String>> {
        let request_headers = match request_headers {
            Some(h) if !h.is_empty() => h,
            _ => return None,
        };

        let allowed_headers = match &self.allowed_headers {
            Some(h) if !h.is_empty() => h,
            _ => return None,
        };

        let allow_any_header = allowed_headers.iter().any(|h| h == ALL);
        let mut result =
            Vec::with_capacity(std::cmp::min(request_headers.len(), allowed_headers.len()));

        for request_header in request_headers {
            if StringUtils::has_text(request_header) {
                let request_header = request_header.trim();
                if allow_any_header {
                    result.push(request_header.to_string());
                } else {
                    for allowed in allowed_headers {
                        if request_header.eq_ignore_ascii_case(allowed) {
                            result.push(request_header.to_string());
                            break;
                        }
                    }
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn trim_trailing_slash(s: &str) -> &str {
        s.strip_suffix('/').unwrap_or(s)
    }
}

static PORTS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(.*):\[(\*|\d+(,\d+)*)]").unwrap());

/// Contains both the user-declared pattern (for example, "https://*.domain.com") and the regex Pattern derived from it.
#[derive(Clone)]
struct OriginPattern {
    declared_pattern: String,
    pattern: Regex,
}

impl OriginPattern {
    /// Creates a new OriginPattern from a pattern string.
    /// Examples: "https://*.domain.com", "https://*.domain.com:[8080,8081]"
    fn new(declared_pattern: impl Into<String>) -> CorsResult<Self> {
        let declared_pattern = declared_pattern.into();
        let compiled = Self::compile_pattern(&declared_pattern)?;
        Ok(OriginPattern {
            declared_pattern,
            pattern: compiled,
        })
    }

    fn compile_pattern(pattern_value: &str) -> CorsResult<Regex> {
        let mut port_list = None;
        let pattern_value = match PORTS_PATTERN.captures(pattern_value) {
            Some(caps) => {
                port_list = Some(caps[2].to_string());
                caps[1].to_string()
            }
            None => pattern_value.to_string(),
        };

        // Escape special regex characters and replace * with .*
        let mut escaped = String::new();
        for ch in pattern_value.chars() {
            if ch == '*' {
                escaped.push_str(".*");
            } else {
                escaped.push_str(&regex::escape(&ch.to_string()));
            }
        }

        // Add port pattern if present
        let port_pattern = if let Some(ports) = port_list {
            if ports == "*" {
                "(:\\d+)?".to_string()
            } else {
                format!(":({})", ports.replace(',', "|"))
            }
        } else {
            String::new()
        };

        let full_pattern = format!("^{}{}$", escaped, port_pattern);
        Regex::new(&full_pattern).map_err(|_| CorsError::InvalidOriginPattern(pattern_value))
    }

    fn declared_pattern(&self) -> &str {
        &self.declared_pattern
    }

    fn matches(&self, origin: &str) -> bool {
        self.pattern.is_match(origin)
    }
}

impl PartialEq for OriginPattern {
    fn eq(&self, other: &Self) -> bool {
        self.declared_pattern == other.declared_pattern
    }
}

impl Hash for OriginPattern {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.declared_pattern.hash(state);
    }
}

impl Eq for OriginPattern {}

impl std::fmt::Debug for OriginPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginPattern")
            .field("declared_pattern", &self.declared_pattern)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = CorsConfiguration::default();
        assert!(config.allowed_origins().is_none());
        assert!(config.allowed_methods().is_none());
        assert!(config.allowed_headers().is_none());
        assert!(config.exposed_headers().is_none());
        assert!(config.allow_credentials().is_none());
        assert!(config.allow_private_network().is_none());
        assert!(config.max_age().is_none());
    }

    #[test]
    fn test_apply_permit_default_values() {
        let mut config = CorsConfiguration::default();
        config.apply_permit_default_values();

        assert_eq!(
            config.allowed_origins(),
            Some(vec![ALL.to_string()].as_slice())
        );
        assert_eq!(
            config.allowed_methods(),
            Some(vec!["GET".to_string(), "HEAD".to_string(), "POST".to_string()].as_slice())
        );
        assert_eq!(
            config.allowed_headers(),
            Some(vec![ALL.to_string()].as_slice())
        );
        assert_eq!(config.max_age(), Some(1800));
    }

    #[test]
    fn test_add_allowed_origin() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_origin("https://example.com").unwrap();
        config.add_allowed_origin("https://another.com").unwrap();

        assert_eq!(
            config.allowed_origins(),
            Some(
                vec![
                    "https://example.com".to_string(),
                    "https://another.com".to_string()
                ]
                .as_slice()
            )
        );
    }

    #[test]
    fn test_add_allowed_origin_with_trailing_slash() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_origin("https://example.com/").unwrap();

        assert_eq!(
            config.allowed_origins(),
            Some(vec!["https://example.com".to_string()].as_slice())
        );
    }

    #[test]
    fn test_add_allowed_origin_comma_delimited() {
        let mut config = CorsConfiguration::default();
        config
            .add_allowed_origin("https://a.com,https://b.com")
            .unwrap();

        assert_eq!(
            config.allowed_origins(),
            Some(vec!["https://a.com".to_string(), "https://b.com".to_string()].as_slice())
        );
    }

    #[test]
    fn test_add_allowed_method() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_method(HttpMethod::GET).unwrap();
        config.add_allowed_method(HttpMethod::POST).unwrap();

        let methods = config.allowed_methods().unwrap();
        assert_eq!(methods, vec!["GET", "POST"]);
    }

    #[test]
    fn test_check_origin_exact_match() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_origin("https://example.com").unwrap();

        assert_eq!(
            config.check_origin(Some("https://example.com")).unwrap(),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            config.check_origin(Some("https://other.com")).unwrap(),
            None
        );
    }

    #[test]
    fn test_check_origin_wildcard() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_origin("*").unwrap();

        assert_eq!(
            config.check_origin(Some("https://anything.com")).unwrap(),
            Some("*".to_string())
        );
    }

    #[test]
    fn test_check_origin_pattern() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_origin_pattern("https://*.example.com");

        assert_eq!(
            config
                .check_origin(Some("https://api.example.com"))
                .unwrap(),
            Some("https://api.example.com".to_string())
        );
        assert_eq!(
            config.check_origin(Some("https://other.com")).unwrap(),
            None
        );
    }

    #[test]
    fn test_check_origin_pattern_with_port() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_origin_pattern("https://*.example.com:[8080,9090]");

        assert_eq!(
            config
                .check_origin(Some("https://api.example.com:8080"))
                .unwrap(),
            Some("https://api.example.com:8080".to_string())
        );
        assert_eq!(
            config
                .check_origin(Some("https://api.example.com:9090"))
                .unwrap(),
            Some("https://api.example.com:9090".to_string())
        );
        assert_eq!(
            config
                .check_origin(Some("https://api.example.com:3000"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn test_check_http_method() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_method(HttpMethod::GET).unwrap();
        config.add_allowed_method(HttpMethod::POST).unwrap();

        let result = config.check_http_method(Some(&HttpMethod::GET));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![HttpMethod::GET, HttpMethod::POST]);

        assert!(config.check_http_method(Some(&HttpMethod::PUT)).is_none());
    }

    #[test]
    fn test_check_http_method_wildcard() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_method_with_str("*").unwrap();

        let result = config.check_http_method(Some(&HttpMethod::PATCH));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![HttpMethod::PATCH]);
    }

    #[test]
    fn test_check_headers() {
        let mut config = CorsConfiguration::default();
        config.add_allowed_header("X-Custom-Header");
        config.add_allowed_header("Content-Type");

        let headers = vec!["X-Custom-Header".to_string(), "Content-Type".to_string()];
        let result = config.check_headers(Some(&headers));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), headers);

        let bad_headers = vec!["X-Other-Header".to_string()];
        assert!(config.check_headers(Some(&bad_headers)).is_none());
    }

    #[test]
    fn test_check_headers_wildcard() {
        let mut config = CorsConfiguration::default();
        config.set_allowed_headers(Some(vec![ALL.to_string()]));

        let headers = vec!["Any-Header".to_string(), "Another-Header".to_string()];
        let result = config.check_headers(Some(&headers));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), headers);
    }

    #[test]
    fn test_validate_allow_credentials_with_wildcard() {
        let mut config = CorsConfiguration::default();
        config
            .set_allowed_origins(Some(vec![ALL.to_string()]))
            .unwrap();
        config.set_allow_credentials(Some(true));

        assert!(config.validate_allow_credentials().is_err());
    }

    #[test]
    fn test_validate_allow_credentials_ok() {
        let mut config = CorsConfiguration::default();
        config
            .set_allowed_origins(Some(vec!["https://example.com".to_string()]))
            .unwrap();
        config.set_allow_credentials(Some(true));

        assert!(config.validate_allow_credentials().is_ok());
    }

    #[test]
    fn test_combine() {
        let mut config1 = CorsConfiguration::default();
        config1.add_allowed_origin("https://a.com").unwrap();
        config1.set_allow_credentials(Some(false));

        let mut config2 = CorsConfiguration::default();
        config2.add_allowed_origin("https://b.com").unwrap();
        config2.set_allow_credentials(Some(true));
        config2.set_max_age(Duration::from_secs(3600));

        let combined = config1.combine(Some(&config2));

        // Origins should be combined
        let origins = combined.allowed_origins().unwrap();
        assert!(origins.contains(&"https://a.com".to_string()));
        assert!(origins.contains(&"https://b.com".to_string()));

        // allow_credentials should be overridden by config2
        assert_eq!(combined.allow_credentials(), Some(true));

        // max_age should be overridden by config2
        assert_eq!(combined.max_age(), Some(3600));
    }

    #[test]
    fn test_from_str_http_method() {
        assert_eq!(HttpMethod::from_str("GET").unwrap(), HttpMethod::GET);
        assert_eq!(HttpMethod::from_str("POST").unwrap(), HttpMethod::POST);
        assert!(HttpMethod::from_str("INVALID").is_err());
    }
}
