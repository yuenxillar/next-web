//! A strict [`HttpFirewall`] implementation.
//!
//! This is a port of Next Security's `StrictHttpFirewall`. It rejects
//! suspicious requests with a [`RequestRejectedError`] instead of trying to
//! normalize them.
//!
//! The following rules are applied:
//!
//! * HTTP methods that are not allowed are rejected, which blocks HTTP verb
//!   tampering and XST attacks. See
//!   [`StrictHttpFirewall::set_allowed_http_methods`].
//! * URLs that are not normalized (they contain `.` or `..` path segments) are
//!   rejected. There is no way to disable this, as it is considered extremely
//!   risky to disable.
//! * URLs that contain characters other than printable ASCII are rejected.
//!   There is no way to disable this either.
//! * URLs that contain a blocked sequence (`;`, `%2f`, `//`, `\`, `%00`,
//!   `%25`, an encoded period, a line or paragraph separator, ...) are
//!   rejected. The entries are tested against the encoded request target and
//!   against its percent-decoded form.
//! * Hosts that the allowed hostnames predicate rejects are rejected.
//! * Header and parameter names and values that their predicates reject are
//!   rejected.
//!
//! # Deviation from Next Security
//!
//! Next validates header and parameter fields lazily, when the application
//! reads them, because the servlet accessors can throw a
//! `RequestRejectedException`. The [`HttpRequest`] accessors of this crate are
//! infallible and cannot report a rejection, so this port validates those
//! fields eagerly in [`HttpFirewall::validate_request`]. The set of rejected
//! requests is therefore a superset of Next's: a request carrying a field
//! that Next would only reject once it is read is rejected before the filter
//! chain runs.

use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

use next_web_core::http::{HttpRequest, HttpResponse};
use next_web_core::Regex;

use crate::web::firewall::request_rejected_error::RequestRejectedError;

use super::http_firewall::{FirewalledRequest, HttpFirewall};

/// A predicate deciding whether a hostname, a header name, a header value, a
/// parameter name, or a parameter value is allowed.
pub type StringPredicate = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// The default predicate for header and parameter names.
///
/// Every character must be assigned and must not be an ISO control character.
pub const ALLOWED_HEADER_NAMES: fn(&str) -> bool = is_assigned_and_not_iso_control;

/// The default predicate for header values.
///
/// Every character must be assigned; ISO control characters are rejected
/// except for the tab, which is allowed inside a header value.
pub const ALLOWED_HEADER_VALUES: fn(&str) -> bool = is_assigned_and_not_iso_control_or_tab;

/// The default predicate for parameter names.
///
/// Every character must be assigned and must not be an ISO control character.
pub const ALLOWED_PARAMETER_NAMES: fn(&str) -> bool = is_assigned_and_not_iso_control;

/// The default predicate for parameter values, which allows any value.
pub const ALLOWED_PARAMETER_VALUES: fn(&str) -> bool = allow_any_value;

/// `%25`, the URL encoded form of `%`.
const ENCODED_PERCENT: &str = "%25";

/// `%`, which appears in a decoded target when `%25` was encoded twice.
const PERCENT: &str = "%";

const FORBIDDEN_ENCODED_PERIOD: [&str; 2] = ["%2e", "%2E"];
const FORBIDDEN_SEMICOLON: [&str; 3] = [";", "%3b", "%3B"];
const FORBIDDEN_FORWARD_SLASH: [&str; 2] = ["%2f", "%2F"];
const FORBIDDEN_DOUBLE_FORWARD_SLASH: [&str; 5] = ["//", "%2f%2f", "%2f%2F", "%2F%2f", "%2F%2F"];
const FORBIDDEN_BACKSLASH: [&str; 3] = ["\\", "%5c", "%5C"];
const FORBIDDEN_NULL: [&str; 2] = ["\0", "%00"];
const FORBIDDEN_LINE_FEED: [&str; 3] = ["\n", "%0a", "%0A"];
const FORBIDDEN_CARRIAGE_RETURN: [&str; 3] = ["\r", "%0d", "%0D"];
const FORBIDDEN_LINE_SEPARATOR: [&str; 1] = ["\u{2028}"];
const FORBIDDEN_PARAGRAPH_SEPARATOR: [&str; 1] = ["\u{2029}"];

/// The HTTP methods allowed by [`StrictHttpFirewall::default`].
const DEFAULT_HTTP_METHODS: [&str; 7] =
    ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"];

/// A strict [`HttpFirewall`] that rejects any suspicious request with a
/// [`RequestRejectedError`].
///
/// The firewall is configured with the Next Security defaults. Use the
/// `set_*` methods to relax or tighten a rule before installing the firewall on
/// a [`FilterChainProxy`](crate::web::FilterChainProxy).
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_security::web::firewall::strict_http_firewall::StrictHttpFirewall;
///
/// let mut firewall = StrictHttpFirewall::default();
/// firewall.set_allowed_http_methods(["GET", "POST"]);
/// assert_eq!(firewall.allowed_http_methods().len(), 2);
/// ```
#[derive(Clone)]
pub struct StrictHttpFirewall {
    /// Blocked sequences tested against the encoded request target.
    encoded_url_blocklist: HashSet<String>,
    /// Blocked sequences tested against the percent-decoded request target.
    decoded_url_blocklist: HashSet<String>,
    /// The allowed HTTP methods, meaningful only when `allow_any_http_method`
    /// is `false`.
    allowed_http_methods: HashSet<String>,
    /// Whether every HTTP method is allowed.
    allow_any_http_method: bool,
    /// Tests the server name taken from the `Host` header.
    allowed_hostnames: StringPredicate,
    /// Tests every header name of the request.
    allowed_header_names: StringPredicate,
    /// Tests every header value of the request.
    allowed_header_values: StringPredicate,
    /// Tests every parameter name of the request.
    allowed_parameter_names: StringPredicate,
    /// Tests every parameter value of the request.
    allowed_parameter_values: StringPredicate,
}

impl StrictHttpFirewall {
    /// Creates a firewall configured with the Next Security defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether any HTTP method is allowed.
    ///
    /// When `allow` is `true`, no validation of the HTTP method is performed,
    /// which can open the application up to HTTP verb tampering and XST
    /// attacks. When `allow` is `false` the default methods are restored.
    pub fn set_unsafe_allow_any_http_method(&mut self, allow: bool) {
        self.allow_any_http_method = allow;
        // Next stores the sentinel `ALLOW_ANY_HTTP_METHOD` (the empty set) in
        // this case; `allow_any_http_method` is what distinguishes it from a
        // caller-supplied empty set, which rejects every method.
        self.allowed_http_methods = if allow {
            HashSet::new()
        } else {
            default_allowed_http_methods()
        };
    }

    /// Determines which HTTP methods should be allowed.
    ///
    /// The default is to allow `DELETE`, `GET`, `HEAD`, `OPTIONS`, `PATCH`,
    /// `POST`, and `PUT`. The comparison is case-sensitive. Passing an empty
    /// collection rejects every method, it does not allow them all.
    pub fn set_allowed_http_methods<I, S>(&mut self, methods: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_http_methods = methods.into_iter().map(Into::into).collect();
        self.allow_any_http_method = false;
    }

    /// Returns the allowed HTTP methods.
    pub fn allowed_http_methods(&self) -> &HashSet<String> {
        &self.allowed_http_methods
    }

    /// Returns whether every HTTP method is allowed.
    pub fn is_allow_any_http_method(&self) -> bool {
        self.allow_any_http_method
    }

    /// Determines if a semicolon is allowed in the URL (that is, matrix
    /// variables).
    ///
    /// The default is to reject semicolons because they are a common way of
    /// performing reflected file download attacks and of bypassing URL based
    /// security (see CVE-2016-5007, CVE-2016-9879, and CVE-2018-1199). It is
    /// also common to put a session id in the path with `;jsessionid=...`,
    /// which leaks it; use cookies instead.
    pub fn set_allow_semicolon(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_SEMICOLON);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_SEMICOLON);
        }
    }

    /// Determines if a URL encoded slash (`%2F`) is allowed in the path.
    ///
    /// The default is to reject it because servlet containers parse it
    /// inconsistently, which can bypass URL based security constraints.
    pub fn set_allow_url_encoded_slash(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_FORWARD_SLASH);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_FORWARD_SLASH);
        }
    }

    /// Determines if a URL encoded double slash (`%2F%2F`) is allowed in the
    /// path.
    ///
    /// The default is to reject it.
    pub fn set_allow_url_encoded_double_slash(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_DOUBLE_FORWARD_SLASH);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_DOUBLE_FORWARD_SLASH);
        }
    }

    /// Determines if a URL encoded period (`%2E`) is allowed in the path.
    ///
    /// The default is to reject it because an encoded period can bypass
    /// security constraints through a directory traversal attack.
    pub fn set_allow_url_encoded_period(&mut self, allow: bool) {
        if allow {
            for value in FORBIDDEN_ENCODED_PERIOD {
                self.encoded_url_blocklist.remove(value);
            }
        } else {
            for value in FORBIDDEN_ENCODED_PERIOD {
                self.encoded_url_blocklist.insert(value.to_string());
            }
        }
    }

    /// Determines if a backslash (`\`) or a URL encoded backslash (`%5C`) is
    /// allowed in the path.
    ///
    /// The default is to reject both.
    pub fn set_allow_back_slash(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_BACKSLASH);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_BACKSLASH);
        }
    }

    /// Determines if a null character (`\0`) or a URL encoded null (`%00`) is
    /// allowed in the path.
    ///
    /// The default is to reject both.
    pub fn set_allow_null(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_NULL);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_NULL);
        }
    }

    /// Determines if a URL encoded percent (`%25`) is allowed in the path.
    ///
    /// The default is to reject it because it can be used to double encode a
    /// request and bypass security constraints.
    pub fn set_allow_url_encoded_percent(&mut self, allow: bool) {
        if allow {
            self.encoded_url_blocklist.remove(ENCODED_PERCENT);
            self.decoded_url_blocklist.remove(PERCENT);
        } else {
            self.encoded_url_blocklist
                .insert(ENCODED_PERCENT.to_string());
            self.decoded_url_blocklist.insert(PERCENT.to_string());
        }
    }

    /// Determines if a URL encoded carriage return (`%0D`) is allowed.
    ///
    /// The default is to reject it.
    pub fn set_allow_url_encoded_carriage_return(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_CARRIAGE_RETURN);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_CARRIAGE_RETURN);
        }
    }

    /// Determines if a URL encoded line feed (`%0A`) is allowed.
    ///
    /// The default is to reject it.
    pub fn set_allow_url_encoded_line_feed(&mut self, allow: bool) {
        if allow {
            self.remove_from_both_blocklists(&FORBIDDEN_LINE_FEED);
        } else {
            self.add_to_both_blocklists(&FORBIDDEN_LINE_FEED);
        }
    }

    /// Determines if a URL encoded line separator is allowed.
    ///
    /// The default is to reject it.
    pub fn set_allow_url_encoded_line_separator(&mut self, allow: bool) {
        if allow {
            self.remove_from_decoded_blocklist(&FORBIDDEN_LINE_SEPARATOR);
        } else {
            self.add_to_decoded_blocklist(&FORBIDDEN_LINE_SEPARATOR);
        }
    }

    /// Determines if a URL encoded paragraph separator is allowed.
    ///
    /// The default is to reject it.
    pub fn set_allow_url_encoded_paragraph_separator(&mut self, allow: bool) {
        if allow {
            self.remove_from_decoded_blocklist(&FORBIDDEN_PARAGRAPH_SEPARATOR);
        } else {
            self.add_to_decoded_blocklist(&FORBIDDEN_PARAGRAPH_SEPARATOR);
        }
    }

    /// Determines which hostnames should be allowed.
    ///
    /// The default is to allow any hostname. The predicate is applied to the
    /// server name taken from the `Host` header.
    pub fn set_allowed_hostnames(
        &mut self,
        predicate: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) {
        self.allowed_hostnames = Arc::new(predicate);
    }

    /// Determines which header names should be allowed.
    ///
    /// The default is [`ALLOWED_HEADER_NAMES`].
    pub fn set_allowed_header_names(
        &mut self,
        predicate: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) {
        self.allowed_header_names = Arc::new(predicate);
    }

    /// Determines which header values should be allowed.
    ///
    /// The default is [`ALLOWED_HEADER_VALUES`].
    pub fn set_allowed_header_values(
        &mut self,
        predicate: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) {
        self.allowed_header_values = Arc::new(predicate);
    }

    /// Determines which parameter names should be allowed.
    ///
    /// The default is [`ALLOWED_PARAMETER_NAMES`].
    pub fn set_allowed_parameter_names(
        &mut self,
        predicate: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) {
        self.allowed_parameter_names = Arc::new(predicate);
    }

    /// Determines which parameter values should be allowed.
    ///
    /// The default is [`ALLOWED_PARAMETER_VALUES`], which allows any value.
    pub fn set_allowed_parameter_values(
        &mut self,
        predicate: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) {
        self.allowed_parameter_values = Arc::new(predicate);
    }

    /// Returns the blocklist tested against the encoded request target.
    ///
    /// The set can be modified in place through
    /// [`StrictHttpFirewall::encoded_url_blocklist_mut`].
    pub fn encoded_url_blocklist(&self) -> &HashSet<String> {
        &self.encoded_url_blocklist
    }

    /// Returns the blocklist tested against the percent-decoded request target.
    pub fn decoded_url_blocklist(&self) -> &HashSet<String> {
        &self.decoded_url_blocklist
    }

    /// Returns the mutable blocklist tested against the encoded request target.
    pub fn encoded_url_blocklist_mut(&mut self) -> &mut HashSet<String> {
        &mut self.encoded_url_blocklist
    }

    /// Returns the mutable blocklist tested against the percent-decoded request
    /// target.
    pub fn decoded_url_blocklist_mut(&mut self) -> &mut HashSet<String> {
        &mut self.decoded_url_blocklist
    }

    /// Returns the blocklist tested against the encoded request target.
    #[deprecated(note = "use `encoded_url_blocklist` instead")]
    pub fn encoded_url_blacklist(&self) -> &HashSet<String> {
        self.encoded_url_blocklist()
    }

    /// Returns the blocklist tested against the percent-decoded request target.
    pub fn decoded_url_blacklist(&self) -> &HashSet<String> {
        self.decoded_url_blocklist()
    }

    /// Adds `values` to both the encoded and the decoded blocklists.
    fn add_to_both_blocklists(&mut self, values: &[&str]) {
        for value in values {
            self.encoded_url_blocklist.insert((*value).to_string());
            self.decoded_url_blocklist.insert((*value).to_string());
        }
    }

    /// Removes `values` from both the encoded and the decoded blocklists.
    fn remove_from_both_blocklists(&mut self, values: &[&str]) {
        for value in values {
            self.encoded_url_blocklist.remove(*value);
            self.decoded_url_blocklist.remove(*value);
        }
    }

    /// Adds `values` to the decoded blocklist only.
    fn add_to_decoded_blocklist(&mut self, values: &[&str]) {
        for value in values {
            self.decoded_url_blocklist.insert((*value).to_string());
        }
    }

    /// Removes `values` from the decoded blocklist only.
    fn remove_from_decoded_blocklist(&mut self, values: &[&str]) {
        for value in values {
            self.decoded_url_blocklist.remove(*value);
        }
    }

    /// Rejects the request when its HTTP method is not allowed.
    fn reject_forbidden_http_method(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<(), RequestRejectedError> {
        if self.allow_any_http_method {
            return Ok(());
        }

        let method = request.method();
        if !self.allowed_http_methods.contains(method.as_str()) {
            return Err(RequestRejectedError(format!(
                "The request was rejected because the HTTP method \"{}\" was not included within the list of allowed HTTP methods {}",
                method,
                format_methods(&self.allowed_http_methods)
            )));
        }
        Ok(())
    }

    /// Rejects the request when the URL contains a blocklisted sequence.
    fn reject_blocklisted_urls(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<(), RequestRejectedError> {
        if !self.encoded_url_blocklist.is_empty() {
            let context_path = request.context_path();
            let path = request.path();
            for forbidden in &self.encoded_url_blocklist {
                if value_contains(context_path, forbidden) || path.contains(forbidden.as_str()) {
                    return Err(rejected_url_error(forbidden));
                }
            }
        }

        if !self.decoded_url_blocklist.is_empty() {
            let decoded = decoded_servlet_path(request);
            for forbidden in &self.decoded_url_blocklist {
                if value_contains(decoded.as_deref(), forbidden) {
                    return Err(rejected_url_error(forbidden));
                }
            }
        }

        Ok(())
    }

    /// Rejects the request when the `Host` header is not trusted.
    fn reject_untrusted_hosts(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<(), RequestRejectedError> {
        if let Some(server_name) = request.server_name() {
            if !(self.allowed_hostnames)(&server_name) {
                return Err(RequestRejectedError(format!(
                    "The request was rejected because the domain {server_name} is untrusted."
                )));
            }
        }
        Ok(())
    }

    /// Rejects the request when a header name or value is not allowed.
    fn reject_not_allowed_headers(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<(), RequestRejectedError> {
        for (name, value) in request.headers().iter() {
            let name = name.as_str();
            if !(self.allowed_header_names)(name) {
                return Err(RequestRejectedError(format!(
                    "The request was rejected because the header name \"{name}\" is not allowed."
                )));
            }
            if let Ok(value) = value.to_str() {
                if !(self.allowed_header_values)(value) {
                    return Err(RequestRejectedError(format!(
                        "The request was rejected because the header: \"{name} \" has a value \"{value}\" that is not allowed."
                    )));
                }
            }
        }
        Ok(())
    }

    /// Rejects the request when a parameter name or value is not allowed.
    fn reject_not_allowed_parameters(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<(), RequestRejectedError> {
        let Some(parameters) = request.parameters() else {
            return Ok(());
        };

        for (name, value) in parameters {
            let name = name.as_ref();
            let value = value.as_ref();
            if !(self.allowed_parameter_names)(name) {
                return Err(RequestRejectedError(format!(
                    "The request was rejected because the parameter name \"{name}\" is not allowed."
                )));
            }
            if !(self.allowed_parameter_values)(value) {
                return Err(RequestRejectedError(format!(
                    "The request was rejected because the parameter: \"{name} \" has a value \"{value}\" that is not allowed."
                )));
            }
        }
        Ok(())
    }
}

impl Default for StrictHttpFirewall {
    fn default() -> Self {
        let allowed_hostnames: StringPredicate = Arc::new(|_: &str| true);
        let allowed_header_names: StringPredicate = Arc::new(ALLOWED_HEADER_NAMES);
        let allowed_header_values: StringPredicate = Arc::new(ALLOWED_HEADER_VALUES);
        let allowed_parameter_names: StringPredicate = Arc::new(ALLOWED_PARAMETER_NAMES);
        let allowed_parameter_values: StringPredicate = Arc::new(ALLOWED_PARAMETER_VALUES);

        let mut firewall = Self {
            encoded_url_blocklist: HashSet::new(),
            decoded_url_blocklist: HashSet::new(),
            allowed_http_methods: default_allowed_http_methods(),
            allow_any_http_method: false,
            allowed_hostnames,
            allowed_header_names,
            allowed_header_values,
            allowed_parameter_names,
            allowed_parameter_values,
        };

        firewall.add_to_both_blocklists(&FORBIDDEN_SEMICOLON);
        firewall.add_to_both_blocklists(&FORBIDDEN_FORWARD_SLASH);
        firewall.add_to_both_blocklists(&FORBIDDEN_DOUBLE_FORWARD_SLASH);
        firewall.add_to_both_blocklists(&FORBIDDEN_BACKSLASH);
        firewall.add_to_both_blocklists(&FORBIDDEN_NULL);
        firewall.add_to_both_blocklists(&FORBIDDEN_LINE_FEED);
        firewall.add_to_both_blocklists(&FORBIDDEN_CARRIAGE_RETURN);

        firewall
            .encoded_url_blocklist
            .insert(ENCODED_PERCENT.to_string());
        for value in FORBIDDEN_ENCODED_PERIOD {
            firewall.encoded_url_blocklist.insert(value.to_string());
        }
        firewall.decoded_url_blocklist.insert(PERCENT.to_string());
        firewall.add_to_decoded_blocklist(&FORBIDDEN_LINE_SEPARATOR);
        firewall.add_to_decoded_blocklist(&FORBIDDEN_PARAGRAPH_SEPARATOR);

        firewall
    }
}

impl HttpFirewall for StrictHttpFirewall {
    /// Validates the request against every rule of the firewall.
    ///
    /// The checks run in the same order as Next's
    /// `StrictHttpFirewall#getFirewalledRequest`: the HTTP method, the
    /// blocklisted URL sequences, the host, the normalization of the URL, the
    /// printable ASCII check of the request target, and finally the header and
    /// parameter predicates.
    fn validate_request(&self, request: &dyn HttpRequest) -> Result<(), RequestRejectedError> {
        self.reject_forbidden_http_method(request)?;
        self.reject_blocklisted_urls(request)?;
        self.reject_untrusted_hosts(request)?;

        if !request_is_normalized(request) {
            return Err(RequestRejectedError(
                "The request was rejected because the URL was not normalized.".to_string(),
            ));
        }

        if !contains_only_printable_ascii_characters(request.path()) {
            return Err(RequestRejectedError(
                "The requestURI was rejected because it can only contain printable ASCII characters."
                    .to_string(),
            ));
        }

        self.reject_not_allowed_headers(request)?;
        self.reject_not_allowed_parameters(request)?;

        Ok(())
    }

    fn get_firewalled_request<'a>(
        &self,
        request: &'a mut dyn HttpRequest,
    ) -> Box<dyn FirewalledRequest + 'a> {
        Box::new(StrictFirewalledRequest { request })
    }

    /// Provides the response which will be passed through the filter chain.
    ///
    /// Next wraps the response in a `FirewalledResponse`; the trait of this
    /// crate returns the original response, so this is a pass-through.
    fn get_firewalled_response<'a>(
        &self,
        response: &'a mut dyn HttpResponse,
    ) -> Box<&'a mut dyn HttpResponse> {
        Box::new(response)
    }
}

/// Request returned after strict validation succeeds.
///
/// `StrictHttpFirewall` validates without rewriting request fields, so `reset`
/// is intentionally a no-op. Keeping the wrapper lifecycle explicit matches
/// Next's contract and allows normalization to be added safely.
pub struct StrictFirewalledRequest<'a> {
    request: &'a mut dyn HttpRequest,
}

impl FirewalledRequest for StrictFirewalledRequest<'_> {
    fn request_mut(&mut self) -> &mut dyn HttpRequest {
        self.request
    }

    fn reset(&mut self) {}
}

/// Returns the allowed HTTP methods of a fresh firewall.
fn default_allowed_http_methods() -> HashSet<String> {
    DEFAULT_HTTP_METHODS
        .iter()
        .copied()
        .map(str::to_owned)
        .collect()
}

/// Renders a set of methods the way Java's `HashSet` `toString` does.
fn format_methods(methods: &HashSet<String>) -> String {
    let mut names: Vec<&str> = methods.iter().map(String::as_str).collect();
    names.sort_unstable();
    format!("[{}]", names.join(", "))
}

/// Builds the error raised for a blocklisted URL sequence.
fn rejected_url_error(forbidden: &str) -> RequestRejectedError {
    RequestRejectedError(format!(
        "The request was rejected because the URL contained a potentially malicious String \"{forbidden}\""
    ))
}

/// Returns whether `value` is not `null` and contains `contains`.
fn value_contains(value: Option<&str>, contains: &str) -> bool {
    value.is_some_and(|value| value.contains(contains))
}

/// Returns the percent-decoded servlet path of the request.
///
/// Next's `getServletPath` and `getPathInfo` are decoded by the servlet
/// container and exclude the context path; this reproduces both properties for
/// the crate's raw [`HttpRequest::path`].
fn decoded_servlet_path(request: &dyn HttpRequest) -> Option<String> {
    let path = request.path();
    let servlet_path = match request.context_path() {
        Some(context_path) if !context_path.is_empty() && path.starts_with(context_path) => {
            &path[context_path.len()..]
        }
        _ => path,
    };

    urlencoding::decode(servlet_path)
        .ok()
        .map(|decoded| decoded.into_owned())
}

/// Returns whether every path of the request is normalized.
fn request_is_normalized(request: &dyn HttpRequest) -> bool {
    if !is_normalized(request.path()) {
        return false;
    }

    if let Some(context_path) = request.context_path() {
        if !is_normalized(context_path) {
            return false;
        }
    }

    // `getServletPath` and `getPathInfo` are decoded by the servlet container,
    // so a traversal sequence can surface only after decoding.
    match decoded_servlet_path(request) {
        Some(decoded) => is_normalized(&decoded),
        None => true,
    }
}

/// Checks whether a path is normalized (it doesn't contain path traversal
/// sequences like `./`, `/../`, or `/.`).
///
/// This is a direct port of Next's algorithm, including its handling of the
/// segment before the first slash.
fn is_normalized(path: &str) -> bool {
    let bytes = path.as_bytes();
    let mut end = bytes.len() as isize;

    while end > 0 {
        let slash_index = bytes[..end as usize]
            .iter()
            .rposition(|&byte| byte == b'/')
            .map_or(-1, |index| index as isize);
        let gap = end - slash_index;

        // ".", "/./" or "/."
        if gap == 2 && bytes.get((slash_index + 1) as usize) == Some(&b'.') {
            return false;
        }
        // "..", "/../" or "/.."
        if gap == 3
            && bytes.get((slash_index + 1) as usize) == Some(&b'.')
            && bytes.get((slash_index + 2) as usize) == Some(&b'.')
        {
            return false;
        }

        end = slash_index;
    }

    true
}

/// Returns whether `value` contains only printable ASCII characters.
fn contains_only_printable_ascii_characters(value: &str) -> bool {
    value
        .chars()
        .all(|character| ('\u{20}'..='\u{7e}').contains(&character))
}

/// A character is allowed when it is assigned and is not an ISO control
/// character.
///
/// This is Next's `ASSIGNED_AND_NOT_ISO_CONTROL_PATTERN`, that is
/// `[\p{IsAssigned}&&[^\p{IsControl}]]` where `IsAssigned` excludes the
/// `Unassigned` (`Cn`) general category and `IsControl` is the `Control` (`Cc`)
/// category.
fn is_assigned_and_not_iso_control(value: &str) -> bool {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\A[^\p{Unassigned}\p{Control}]*\z").expect("the pattern is valid")
    });
    PATTERN.is_match(value)
}

/// A character is allowed when it is assigned, is not an ISO control
/// character, or is a tab.
///
/// This is Next's `HEADER_VALUE_PATTERN`, that is
/// `[\p{IsAssigned}&&[[^\p{IsControl}]||\t]]`.
fn is_assigned_and_not_iso_control_or_tab(value: &str) -> bool {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\A([^\p{Unassigned}\p{Control}]|\t)*\z").expect("the pattern is valid")
    });
    PATTERN.is_match(value)
}

/// The predicate that allows every value, matching Next's default for
/// parameter values.
fn allow_any_value(_value: &str) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, http::header::HOST};

    use super::{
        contains_only_printable_ascii_characters, is_assigned_and_not_iso_control,
        is_assigned_and_not_iso_control_or_tab, is_normalized, StrictHttpFirewall,
        ALLOWED_PARAMETER_VALUES,
    };
    use crate::web::firewall::http_firewall::HttpFirewall;

    fn request(method: &str, uri: &str) -> Request {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn the_default_http_methods_are_allowed() {
        let firewall = StrictHttpFirewall::default();

        for method in ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"] {
            firewall.validate_request(&request(method, "/")).unwrap();
        }

        assert!(firewall.validate_request(&request("TRACE", "/")).is_err());
    }

    #[test]
    fn any_http_method_can_be_allowed() {
        let mut firewall = StrictHttpFirewall::default();
        firewall.set_unsafe_allow_any_http_method(true);
        firewall.validate_request(&request("TRACE", "/")).unwrap();
        assert!(firewall.allowed_http_methods().is_empty());

        firewall.set_unsafe_allow_any_http_method(false);
        assert!(firewall.validate_request(&request("TRACE", "/")).is_err());
    }

    #[test]
    fn an_explicitly_empty_method_list_rejects_every_method() {
        let mut firewall = StrictHttpFirewall::default();
        firewall.set_allowed_http_methods(Vec::<String>::new());

        assert!(firewall.validate_request(&request("GET", "/")).is_err());
    }

    #[test]
    fn blocked_sequences_are_rejected() {
        let firewall = StrictHttpFirewall::default();

        for uri in [
            "/a;b", "/a%3bb", "/a%2fb", "/a//b", "/a%5cb", "/a%00b", "/a%25b", "/a%2eb",
        ] {
            assert!(
                firewall.validate_request(&request("GET", uri)).is_err(),
                "{uri} should be rejected"
            );
        }

        firewall.validate_request(&request("GET", "/a/b")).unwrap();
    }

    #[test]
    fn a_blocked_sequence_can_be_allowed() {
        let mut firewall = StrictHttpFirewall::default();
        assert!(firewall.validate_request(&request("GET", "/a;b")).is_err());

        firewall.set_allow_semicolon(true);
        firewall.validate_request(&request("GET", "/a;b")).unwrap();
        assert!(!firewall.encoded_url_blocklist().contains(";"));
        assert!(!firewall.decoded_url_blocklist().contains(";"));

        firewall.set_allow_semicolon(false);
        assert!(firewall.validate_request(&request("GET", "/a;b")).is_err());
    }

    #[test]
    fn url_encoded_period_and_percent_can_be_allowed() {
        let mut firewall = StrictHttpFirewall::default();
        assert!(firewall
            .validate_request(&request("GET", "/a%2eb"))
            .is_err());
        assert!(firewall
            .validate_request(&request("GET", "/a%25b"))
            .is_err());

        firewall.set_allow_url_encoded_period(true);
        firewall
            .validate_request(&request("GET", "/a%2eb"))
            .unwrap();

        firewall.set_allow_url_encoded_percent(true);
        firewall
            .validate_request(&request("GET", "/a%25b"))
            .unwrap();
    }

    #[test]
    fn urls_that_are_not_normalized_are_rejected() {
        let firewall = StrictHttpFirewall::default();

        for uri in ["/a/../b", "/a/./b", "/.", "/..", "/a/.."] {
            assert!(
                firewall.validate_request(&request("GET", uri)).is_err(),
                "{uri} should be rejected"
            );
        }

        firewall.validate_request(&request("GET", "/a/b")).unwrap();
        firewall.validate_request(&request("GET", "/a..b")).unwrap();
    }

    #[test]
    fn untrusted_hosts_are_rejected() {
        let mut firewall = StrictHttpFirewall::default();
        firewall.set_allowed_hostnames(|hostname| hostname == "example.com");

        let with_host = |host: &str| {
            Request::builder()
                .method("GET")
                .uri("/")
                .header(HOST, host)
                .body(Body::empty())
                .unwrap()
        };

        firewall
            .validate_request(&with_host("example.com"))
            .unwrap();
        assert!(firewall.validate_request(&with_host("evil.com")).is_err());
    }

    #[test]
    fn disallowed_header_and_parameter_fields_are_rejected() {
        let mut firewall = StrictHttpFirewall::default();
        firewall.set_allowed_header_values(|value| value != "blocked");
        firewall.set_allowed_parameter_names(|name| name != "secret");

        let blocked_header = Request::builder()
            .method("GET")
            .uri("/")
            .header("x-test", "blocked")
            .body(Body::empty())
            .unwrap();
        assert!(firewall.validate_request(&blocked_header).is_err());

        assert!(firewall
            .validate_request(&request("GET", "/?secret=1"))
            .is_err());
        firewall
            .validate_request(&request("GET", "/?name=1"))
            .unwrap();
    }

    #[test]
    fn the_firewalled_request_forwards_to_the_original_request() {
        let firewall = StrictHttpFirewall::default();
        let mut request = request("GET", "/a/b");

        let mut wrapped = firewall.get_firewalled_request(&mut request);
        assert_eq!(wrapped.request_mut().path(), "/a/b");
        wrapped.reset();
    }

    #[test]
    fn default_predicates_follow_the_java_assigned_and_control_rules() {
        assert!(is_assigned_and_not_iso_control("header-name"));
        assert!(is_assigned_and_not_iso_control("naïve"));
        assert!(!is_assigned_and_not_iso_control("with\0null"));
        assert!(!is_assigned_and_not_iso_control("with\u{7f}del"));
        assert!(!is_assigned_and_not_iso_control("unassigned\u{378}"));

        assert!(is_assigned_and_not_iso_control_or_tab("value\twith\ttabs"));
        assert!(!is_assigned_and_not_iso_control_or_tab(
            "value\nwith\nbreaks"
        ));
        assert!(ALLOWED_PARAMETER_VALUES("anything at all"));
    }

    #[test]
    fn the_normalization_helper_matches_spring() {
        assert!(is_normalized(""));
        assert!(is_normalized("/"));
        assert!(is_normalized("/a/b"));
        assert!(is_normalized("/a..b"));
        assert!(!is_normalized("."));
        assert!(!is_normalized(".."));
        assert!(!is_normalized("./a"));
        assert!(!is_normalized("/a/."));
        assert!(!is_normalized("/a/.."));
        assert!(!is_normalized("/a/./b"));
        assert!(!is_normalized("/a/../b"));
    }

    #[test]
    fn only_printable_ascii_is_allowed_in_the_request_target() {
        assert!(contains_only_printable_ascii_characters("/a/b"));
        assert!(contains_only_printable_ascii_characters(""));
        assert!(!contains_only_printable_ascii_characters("/a\u{7f}"));
        assert!(!contains_only_printable_ascii_characters("/\u{4e2d}"));
    }
}
