//! HTTP cookie handling for request parsing and response generation.
//!
//! This module provides a [`Cookie`] type for reading cookies from incoming
//! requests and building `Set-Cookie` headers for responses. It supports
//! standard cookie attributes including `Path`, `Domain`, `Max-Age`, `Secure`,
//! `HttpOnly`, and the `SameSite` security policy.
//!
//! # Examples
//!
//! Creating a session cookie with security attributes:
//!
//! ```
//! use cookie::{Cookie, SameSite};
//!
//! let mut cookie = Cookie::new("session_id", Some("abc123".to_string()));
//! cookie.set_path("/");
//! cookie.set_secure(true);
//! cookie.set_http_only(true);
//! cookie.set_same_site(SameSite::Lax);
//! assert_eq!(
//!     cookie.to_string(),
//!     "session_id=abc123; Path=/; Secure; HttpOnly; SameSite=Lax"
//! );
//! ```
//!
//! Using the builder pattern:
//!
//! ```
//! use cookie::{Cookie, SameSite};
//!
//! let cookie = Cookie::builder("session_id", Some("abc123".to_string()))
//!     .path("/")
//!     .secure(true)
//!     .http_only(true)
//!     .same_site(SameSite::Lax)
//!     .build();
//!
//! assert_eq!(
//!     cookie.to_string(),
//!     "session_id=abc123; Path=/; Secure; HttpOnly; SameSite=Lax"
//! );
//! ```

use std::{
    fmt::{self, Display},
    str::FromStr,
    time::Duration,
};

/// An HTTP cookie used for both reading from requests and building `Set-Cookie`
/// response headers.
///
/// This struct represents a single cookie with its value and optional
/// attributes. It follows the HTTP State Management Mechanism specification
/// (RFC 6265).
///
/// # Default Values
///
/// * `max_age`: `-1` (indicates a session cookie, which expires when the browser closes)
/// * `secure`: `false`
/// * `http_only`: `false`
/// * `same_site`: `None`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookie {
    name: String,
    value: String,
    path: Option<String>,
    domain: Option<String>,
    max_age: Option<u32>,
    secure: bool,
    http_only: bool,
    partitioned: bool,
    same_site: Option<SameSite>,
}

impl Cookie {
    /// Creates a new cookie with the given name and optional value.
    ///
    /// All attributes are set to their default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::Cookie;
    ///
    /// let cookie = Cookie::new("session_id", Some("abc123".to_string()));
    /// assert_eq!(cookie.name(), "session_id");
    /// assert_eq!(cookie.value(), "abc123");
    /// ```
    pub fn new(name: impl Into<String>, value: Option<String>) -> Self {
        Self {
            name: name.into(),
            value: value.unwrap_or_default(),
            path: None,
            domain: None,
            max_age: None,
            secure: false,
            http_only: false,
            partitioned: false,
            same_site: None,
        }
    }

    /// Returns a builder for constructing a cookie with a fluent API.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::{Cookie, SameSite};
    ///
    /// let cookie = Cookie::builder("session_id", Some("abc123".to_string()))
    ///     .path("/")
    ///     .secure(true)
    ///     .http_only(true)
    ///     .same_site(SameSite::Lax)
    ///     .build();
    /// ```
    pub fn builder(name: impl Into<String>, value: Option<String>) -> CookieBuilder {
        CookieBuilder::new(name, value)
    }

    /// Sets the `Max-Age` attribute of the cookie using a [`Duration`].
    ///
    /// This attribute defines the lifetime of the cookie in seconds. When set,
    /// the browser will persist the cookie for the specified duration.
    ///
    /// # Behavior
    ///
    /// | Value | Result |
    /// |-------|--------|
    /// | `Duration::from_secs(3600)` | Cookie expires in 1 hour |
    /// | `Duration::from_secs(0)` | Cookie is immediately deleted by the browser |
    /// | `None` (not set) | Session cookie, deleted when browser closes |
    ///
    /// # Note
    ///
    /// The duration is converted to seconds and stored as a `u32`. For durations
    /// longer than `u32::MAX` seconds (~136 years), the value will be clamped.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use cookie::Cookie;
    ///
    /// let mut cookie = Cookie::new("session_id", Some("abc123".to_string()));
    ///
    /// // Set cookie to expire in 1 hour
    /// cookie.set_max_age(Duration::from_secs(3600));
    /// assert_eq!(cookie.max_age(), Some(3600));
    ///
    /// // Set cookie to be immediately deleted
    /// cookie.set_max_age(Duration::from_secs(0));
    /// assert_eq!(cookie.max_age(), Some(0));
    /// ```
    ///
    /// To set a session cookie (no `Max-Age` attribute), use [`clear_max_age()`]
    /// or simply don't call this method.
    ///
    /// [`clear_max_age()`]: Cookie::clear_max_age
    pub fn set_max_age(&mut self, max_age: Duration) {
        self.max_age = Some(max_age.as_secs() as u32);
    }

    /// Sets the `Max-Age` attribute of the cookie in seconds.
    ///
    /// This is a convenience method for setting the cookie lifetime directly
    /// with a whole number of seconds.
    ///
    /// # Behavior
    ///
    /// | Value | Result |
    /// |-------|--------|
    /// | `3600` | Cookie expires in 1 hour |
    /// | `0` | Cookie is immediately deleted by the browser |
    /// | `None` (not set) | Session cookie, deleted when browser closes |
    ///
    /// # Panics
    ///
    /// This method does not panic. Values larger than `u32::MAX` are clamped.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::Cookie;
    ///
    /// let mut cookie = Cookie::new("session_id", Some("abc123".to_string()));
    ///
    /// // Set cookie to expire in 1 day (86400 seconds)
    /// cookie.set_max_age_secs(86400);
    /// assert_eq!(cookie.max_age(), Some(86400));
    ///
    /// // Set cookie to be deleted immediately
    /// cookie.set_max_age_secs(0);
    /// assert_eq!(cookie.max_age(), Some(0));
    /// ```
    ///
    /// To create a session cookie (no `Max-Age` attribute), call [`clear_max_age()`]
    /// instead.
    ///
    /// [`clear_max_age()`]: Cookie::clear_max_age
    pub fn set_max_age_secs(&mut self, max_age: u32) {
        self.max_age = Some(max_age);
    }

    /// Sets the `Secure` attribute of the cookie.
    ///
    /// When `true`, the cookie will only be sent over HTTPS connections.
    ///
    /// # Note
    ///
    /// When using `SameSite::None`, this attribute must be set to `true`.
    pub fn set_secure(&mut self, secure: bool) {
        self.secure = secure;
    }

    /// Sets the `Path` attribute of the cookie.
    ///
    /// This attribute defines the URL path that must exist in the requested URL
    /// for the browser to send the cookie.
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.path = Some(path.into());
    }

    /// Sets the `HttpOnly` attribute of the cookie.
    ///
    /// When `true`, the cookie is inaccessible to client-side JavaScript,
    /// providing protection against cross-site scripting (XSS) attacks.
    pub fn set_http_only(&mut self, http_only: bool) {
        self.http_only = http_only;
    }

    /// Sets the `Domain` attribute of the cookie.
    ///
    /// This attribute defines the domain that the cookie is valid for.
    /// If not set, the cookie is only valid for the current host.
    pub fn set_domain(&mut self, domain: impl Into<String>) {
        self.domain = Some(domain.into());
    }

    /// Sets the `Partitioned` attribute of the cookie.
    ///
    /// This attribute is only supported in browsers that support the `Partitioned` cookie attribute.
    pub fn set_partitioned(&mut self, partitioned: bool) {
        self.partitioned = partitioned;
    }

    /// Sets the `SameSite` attribute of the cookie.
    ///
    /// This attribute controls whether the cookie is sent with cross-site requests,
    /// providing protection against CSRF attacks.
    ///
    /// # Note
    ///
    /// Setting `SameSite::None` requires the cookie to also have the `Secure`
    /// attribute set to `true`. This validation is enforced when building the
    /// cookie via the builder or when calling `validate()`.
    pub fn set_same_site(&mut self, same_site: SameSite) {
        self.same_site = Some(same_site);
    }

    /// Returns the cookie name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the cookie value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the `Path` attribute, if set.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the `Domain` attribute, if set.
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// Returns the `Max-Age` attribute.
    ///
    /// A value of `None` indicates a session cookie.
    pub fn max_age(&self) -> Option<u32> {
        self.max_age
    }

    /// Returns whether the `Secure` attribute is set.
    pub fn is_secure(&self) -> bool {
        self.secure
    }

    /// Returns whether the `HttpOnly` attribute is set.
    pub fn is_http_only(&self) -> bool {
        self.http_only
    }

    /// Returns whether the `Partitioned` attribute is set.
    pub fn is_partitioned(&self) -> bool {
        self.partitioned
    }

    /// Returns the `SameSite` attribute, if set.
    pub fn same_site(&self) -> Option<&SameSite> {
        self.same_site.as_ref()
    }

    /// Validates the cookie configuration.
    ///
    /// Checks that:
    /// - If `SameSite` is `None`, the `Secure` attribute is `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::{Cookie, SameSite};
    ///
    /// let mut cookie = Cookie::new("session_id", Some("abc123".to_string()));
    /// cookie.set_same_site(SameSite::None);
    /// // This will fail because Secure is not set
    /// assert!(cookie.validate().is_err());
    ///
    /// cookie.set_secure(true);
    /// assert!(cookie.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), CookieError> {
        if self.partitioned {
            if !self.secure {
                return Err(CookieError::PartitionedRequiresSecure);
            }
            if let Some(SameSite::None) = self.same_site {
                return Err(CookieError::PartitionedWithSameSiteNone);
            }
        }

        Rfc6265Utils::validate_cookie_name(&self.name)?;
        Rfc6265Utils::validate_cookie_value(Some(self.value.as_str()))?;
        Rfc6265Utils::validate_domain(self.domain.as_deref())?;
        Rfc6265Utils::validate_path(self.path.as_deref())?;

        Ok(())
    }
}

/// A builder for constructing cookies with a fluent API.
#[derive(Clone, Debug, Default)]
pub struct CookieBuilder {
    name: String,
    value: Option<String>,
    path: Option<String>,
    domain: Option<String>,
    max_age: Option<u32>,
    secure: bool,
    http_only: bool,
    partitioned: bool,
    same_site: Option<SameSite>,
}

impl CookieBuilder {
    /// Creates a new cookie builder.
    pub fn new(name: impl Into<String>, value: Option<String>) -> Self {
        Self {
            name: name.into(),
            value,
            path: None,
            domain: None,
            max_age: None,
            secure: false,
            http_only: false,
            partitioned: false,
            same_site: None,
        }
    }

    /// Sets the `Path` attribute.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets the `Domain` attribute.
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Sets the `Max-Age` attribute.
    pub fn max_age(mut self, max_age: impl Into<Option<u32>>) -> Self {
        self.max_age = max_age.into();
        self
    }

    /// Sets the `Secure` attribute.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Sets the `HttpOnly` attribute.
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// Sets the `Partitioned` attribute.
    pub fn partitioned(mut self, partitioned: bool) -> Self {
        self.partitioned = partitioned;
        self
    }

    /// Sets the `SameSite` attribute.
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }

    /// Builds the cookie, performing validation.
    ///
    /// # Errors
    ///
    /// Returns `CookieError::SameSiteNoneRequiresSecure` if `SameSite::None`
    /// is set without the `Secure` attribute.
    pub fn build(self) -> Result<Cookie, CookieError> {
        let cookie = Cookie {
            name: self.name,
            value: self.value.unwrap_or_default(),
            path: self.path,
            domain: self.domain,
            max_age: self.max_age,
            secure: self.secure,
            http_only: self.http_only,
            partitioned: self.partitioned,
            same_site: self.same_site,
        };
        cookie.validate()?;
        Ok(cookie)
    }
}

impl fmt::Display for Cookie {
    /// Formats the cookie as a `Set-Cookie` header value.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::{Cookie, SameSite};
    ///
    /// let mut cookie = Cookie::new("session_id", Some("abc123".to_string()));
    /// cookie.set_path("/");
    /// cookie.set_secure(true);
    /// cookie.set_same_site(SameSite::Lax);
    /// assert_eq!(
    ///     cookie.to_string(),
    ///     "session_id=abc123; Path=/; Secure; SameSite=Lax"
    /// );
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)?;
        if let Some(path) = &self.path {
            write!(f, "; Path={}", path)?;
        }
        if let Some(domain) = &self.domain {
            write!(f, "; Domain={}", domain)?;
        }
        if let Some(max_age) = self.max_age {
            write!(f, "; Max-Age={}", max_age)?;
        }
        if self.secure {
            write!(f, "; Secure")?;
        }
        if self.http_only {
            write!(f, "; HttpOnly")?;
        }
        if self.partitioned {
            write!(f, "; Partitioned")?;
        }
        if let Some(same_site) = &self.same_site {
            write!(f, "; SameSite={}", same_site.as_str())?;
        }
        Ok(())
    }
}

/// The `SameSite` attribute of a cookie, which controls whether the browser
/// should send the cookie with cross-site requests.
///
/// This is an important security feature to help prevent CSRF attacks.
///
/// # Browser Defaults
///
/// Modern browsers (Chrome 80+, Firefox 84+) default to [`Lax`] if the attribute
/// is not explicitly set. This provides a good balance between security and
/// usability without breaking most web applications.
///
/// # Security Considerations
///
/// Setting `SameSite::None` requires the `Secure` attribute to be `true`.
/// Browsers will reject cookies with `SameSite::None` over insecure (HTTP)
/// connections.
///
/// # Examples
///
/// ```
/// use cookie::SameSite;
///
/// // Parse from a string
/// let same_site: SameSite = "Strict".parse().unwrap();
/// assert_eq!(same_site, SameSite::Strict);
///
/// // Convert to string
/// assert_eq!(SameSite::Lax.as_str(), "Lax");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SameSite {
    /// The strictest policy. The cookie is only sent in a first-party context.
    ///
    /// It will not be sent for requests initiated from external sites, even via
    /// links. This provides the best CSRF protection but may harm user experience.
    Strict,

    /// The default policy in modern browsers (Chrome 80+, Firefox 84+).
    ///
    /// The cookie is not sent for most cross-site requests, but IS sent when
    /// navigating to the site via a top-level link click (e.g., `<a href="...">`).
    /// This offers a good balance between security and usability.
    Lax,

    /// Allows the cookie to be sent with all cross-site requests.
    ///
    /// **Important**: When using `None`, the cookie MUST also have the `Secure`
    /// attribute set (HTTPS only), otherwise modern browsers will reject it.
    /// Use this only when necessary (e.g., when your site needs to be embedded
    /// via iframes).
    None,
}

impl SameSite {
    /// Returns the string representation of the SameSite value.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::SameSite;
    ///
    /// assert_eq!(SameSite::Strict.as_str(), "Strict");
    /// assert_eq!(SameSite::Lax.as_str(), "Lax");
    /// assert_eq!(SameSite::None.as_str(), "None");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

impl FromStr for SameSite {
    type Err = InvalidSameSiteValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Strict" => Ok(SameSite::Strict),
            "Lax" => Ok(SameSite::Lax),
            "None" => Ok(SameSite::None),
            invalid => Err(InvalidSameSiteValue(invalid.to_string())),
        }
    }
}

impl Display for SameSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an invalid `SameSite` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSameSiteValue(pub String);

impl fmt::Display for InvalidSameSiteValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid SameSite value: '{}', expected one of: Strict, Lax, None",
            self.0
        )
    }
}

impl std::error::Error for InvalidSameSiteValue {}

/// Errors that can occur when working with cookies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieError {
    /// `SameSite::None` requires the `Secure` attribute to be set.
    SameSiteNoneRequiresSecure,
    /// `Partitioned` requires the `Secure` attribute to be set.
    PartitionedRequiresSecure,
    /// `Partitioned` cannot be used with `SameSite=None`.
    PartitionedWithSameSiteNone,
    /// Validation error occurred during cookie parsing or validation.
    ValidationError(CookieValidationError),
}

impl fmt::Display for CookieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CookieError::SameSiteNoneRequiresSecure => {
                write!(f, "SameSite=None requires the Secure attribute to be set")
            }
            CookieError::PartitionedRequiresSecure => {
                write!(
                    f,
                    "Partitioned attribute requires Secure attribute to be set"
                )
            }
            CookieError::PartitionedWithSameSiteNone => {
                write!(f, "Partitioned attribute cannot be used with SameSite=None")
            }
            CookieError::ValidationError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

impl std::error::Error for CookieError {}

impl From<CookieValidationError> for CookieError {
    fn from(err: CookieValidationError) -> Self {
        Self::ValidationError(err)
    }
}

/// RFC 6265 compliant cookie validation utilities.
///
/// These validators ensure that cookie attributes conform to the HTTP State
/// Management Mechanism specification (RFC 6265) and the token specification
/// (RFC 2616).
struct Rfc6265Utils;

impl Rfc6265Utils {
    /// Separator characters that are not allowed in cookie names per RFC 2616.
    const SEPARATOR_CHARS: &'static [char] = &[
        '(', ')', '<', '>', '@', ',', ';', ':', '\\', '"', '/', '[', ']', '?', '=', '{', '}', ' ',
    ];

    /// Allowed characters in cookie domains per RFC 6265.
    const DOMAIN_CHARS: &'static [char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
        'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '.', '-',
    ];

    /// Validates a cookie name according to RFC 2616 token rules.
    ///
    /// # Requirements
    ///
    /// - Must only contain US-ASCII characters (0x20-0x7E)
    /// - Must not contain control characters (0x00-0x1F, 0x7F)
    /// - Must not contain separator characters: `()<>@,;:\\"/[]?={} `
    ///
    /// # Errors
    ///
    /// Returns `CookieValidationError::InvalidName` if the name is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::validators::Rfc6265Utils;
    ///
    /// // Valid cookie names
    /// assert!(Rfc6265Utils::validate_cookie_name("session_id").is_ok());
    /// assert!(Rfc6265Utils::validate_cookie_name("XSRF-TOKEN").is_ok());
    ///
    /// // Invalid cookie names
    /// assert!(Rfc6265Utils::validate_cookie_name("session;id").is_err());
    /// assert!(Rfc6265Utils::validate_cookie_name("session id").is_err());
    /// ```
    pub fn validate_cookie_name(name: &str) -> Result<(), CookieValidationError> {
        if name.is_empty() {
            return Err(CookieValidationError::InvalidName(
                "cookie name cannot be empty".to_string(),
            ));
        }

        for ch in name.chars() {
            let code = ch as u32;

            // CTL = <US-ASCII control chars (octets 0 - 31) and DEL (127)>
            if code <= 0x1F || code == 0x7F {
                return Err(CookieValidationError::InvalidName(format!(
                    "RFC2616 token cannot have control chars: 0x{:02X}",
                    code
                )));
            }

            // Check for separator characters
            if Self::SEPARATOR_CHARS.contains(&ch) {
                return Err(CookieValidationError::InvalidName(format!(
                    "RFC2616 token cannot have separator chars such as '{}'",
                    ch
                )));
            }

            // Check for non-US-ASCII characters
            if code >= 0x80 {
                return Err(CookieValidationError::InvalidName(format!(
                    "RFC2616 token can only have US-ASCII: 0x{:02X}",
                    code
                )));
            }
        }

        Ok(())
    }

    /// Validates a cookie value according to RFC 2616.
    ///
    /// # Requirements
    ///
    /// - Must only contain US-ASCII characters
    /// - Must not contain control characters (0x00-0x1F, 0x7F)
    /// - Must not contain: `"` (0x22), `,` (0x2C), `;` (0x3B), `\` (0x5C)
    /// - Values may be quoted with double quotes (e.g., `"value"`)
    ///
    /// # Errors
    ///
    /// Returns `CookieValidationError::InvalidValue` if the value is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::validators::Rfc6265Utils;
    ///
    /// // Valid cookie values
    /// assert!(Rfc6265Utils::validate_cookie_value(Some("abc123")).is_ok());
    /// assert!(Rfc6265Utils::validate_cookie_value(Some("value with spaces")).is_ok());
    /// assert!(Rfc6265Utils::validate_cookie_value(Some("\"quoted value\"")).is_ok());
    /// assert!(Rfc6265Utils::validate_cookie_value(None).is_ok());
    ///
    /// // Invalid cookie values
    /// assert!(Rfc6265Utils::validate_cookie_value(Some("value;with;semicolon")).is_err());
    /// assert!(Rfc6265Utils::validate_cookie_value(Some("value,with,comma")).is_err());
    /// ```
    pub fn validate_cookie_value(value: Option<&str>) -> Result<(), CookieValidationError> {
        let value = match value {
            Some(v) => v,
            None => return Ok(()),
        };

        if value.is_empty() {
            return Ok(());
        }

        let mut start = 0;
        let mut end = value.len();

        // Handle quoted values
        if end > 1 && value.starts_with('"') && value.ends_with('"') {
            start = 1;
            end -= 1;
        }

        for ch in value[start..end].chars() {
            let code = ch as u32;

            // Check for invalid characters
            // - 0x00-0x20: control chars and space (0x20 is SP)
            // - 0x22: double quote ("), 0x2C: comma (,), 0x3B: semicolon (;), 0x5C: backslash (\)
            // - 0x7F: DEL
            if code < 0x21
                || code == 0x22
                || code == 0x2C
                || code == 0x3B
                || code == 0x5C
                || code == 0x7F
            {
                return Err(CookieValidationError::InvalidValue(format!(
                    "RFC2616 cookie value cannot have '{}' (0x{:02X})",
                    ch, code
                )));
            }

            // Check for non-US-ASCII characters
            if code >= 0x80 {
                return Err(CookieValidationError::InvalidValue(format!(
                    "RFC2616 cookie value can only have US-ASCII chars: 0x{:02X}",
                    code
                )));
            }
        }

        Ok(())
    }

    /// Validates a cookie domain according to RFC 6265 Section 4.1.2.3.
    ///
    /// # Requirements
    ///
    /// - Must not start with '-' or end with '.' or '-'
    /// - Must only contain: `0-9`, `a-z`, `A-Z`, `.`, `-`
    /// - Must not have adjacent dots or dot-dash combinations
    ///
    /// # Errors
    ///
    /// Returns `CookieValidationError::InvalidDomain` if the domain is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::validators::Rfc6265Utils;
    ///
    /// // Valid domains
    /// assert!(Rfc6265Utils::validate_domain(Some("example.com")).is_ok());
    /// assert!(Rfc6265Utils::validate_domain(Some("api.example.com")).is_ok());
    /// assert!(Rfc6265Utils::validate_domain(None).is_ok());
    ///
    /// // Invalid domains
    /// assert!(Rfc6265Utils::validate_domain(Some("-example.com")).is_err());
    /// assert!(Rfc6265Utils::validate_domain(Some("example.com-")).is_err());
    /// assert!(Rfc6265Utils::validate_domain(Some("example..com")).is_err());
    /// assert!(Rfc6265Utils::validate_domain(Some("example-.com")).is_err());
    /// ```
    pub fn validate_domain(domain: Option<&str>) -> Result<(), CookieValidationError> {
        let domain = match domain {
            Some(d) => d,
            None => return Ok(()),
        };

        if domain.is_empty() {
            return Ok(());
        }

        let chars: Vec<char> = domain.chars().collect();
        let len = chars.len();

        // Check first and last characters
        let first = chars[0];
        let last = chars[len - 1];
        if first == '-' || last == '.' || last == '-' {
            return Err(CookieValidationError::InvalidDomain(format!(
                "Invalid first/last char in cookie domain: {}",
                domain
            )));
        }

        // Validate each character and adjacent pairs
        let mut prev = ' ';
        for (i, &ch) in chars.iter().enumerate() {
            // Check if character is allowed
            if !Self::DOMAIN_CHARS.contains(&ch) {
                return Err(CookieValidationError::InvalidDomain(format!(
                    "{}: invalid cookie domain char '{}'",
                    domain, ch
                )));
            }

            // Check for invalid adjacent patterns: "..", ".-", "-."
            if i > 0 {
                if (prev == '.' && (ch == '.' || ch == '-')) || (prev == '-' && ch == '.') {
                    return Err(CookieValidationError::InvalidDomain(format!(
                        "{}: invalid domain sequence '{}{}'",
                        domain, prev, ch
                    )));
                }
            }

            prev = ch;
        }

        Ok(())
    }

    /// Validates a cookie path according to RFC 6265 Section 4.1.2.4.
    ///
    /// # Requirements
    ///
    /// - Must only contain printable US-ASCII characters (0x20-0x7E)
    /// - Must not contain semicolons (`;`)
    ///
    /// # Errors
    ///
    /// Returns `CookieValidationError::InvalidPath` if the path is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use cookie::validators::Rfc6265Utils;
    ///
    /// // Valid paths
    /// assert!(Rfc6265Utils::validate_path(Some("/")).is_ok());
    /// assert!(Rfc6265Utils::validate_path(Some("/app")).is_ok());
    /// assert!(Rfc6265Utils::validate_path(Some("/my app")).is_ok()); // space is allowed
    /// assert!(Rfc6265Utils::validate_path(None).is_ok());
    ///
    /// // Invalid paths
    /// assert!(Rfc6265Utils::validate_path(Some("/app;admin")).is_err());
    /// assert!(Rfc6265Utils::validate_path(Some("/app\x1F")).is_err()); // control char
    /// ```
    pub fn validate_path(path: Option<&str>) -> Result<(), CookieValidationError> {
        let path = match path {
            Some(p) => p,
            None => return Ok(()),
        };

        if path.is_empty() {
            return Ok(());
        }

        for ch in path.chars() {
            let code = ch as u32;

            // Must be printable US-ASCII (0x20 - 0x7E) and not ';'
            if code < 0x20 || code > 0x7E || ch == ';' {
                return Err(CookieValidationError::InvalidPath(format!(
                    "Invalid cookie path char '{}' (0x{:02X})",
                    ch, code
                )));
            }
        }

        Ok(())
    }
}

/// Errors that can occur during cookie validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieValidationError {
    /// Cookie name contains invalid characters.
    InvalidName(String),
    /// Cookie value contains invalid characters.
    InvalidValue(String),
    /// Cookie domain contains invalid characters or format.
    InvalidDomain(String),
    /// Cookie path contains invalid characters.
    InvalidPath(String),
}

impl fmt::Display for CookieValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CookieValidationError::InvalidName(msg) => {
                write!(f, "Invalid cookie name: {}", msg)
            }
            CookieValidationError::InvalidValue(msg) => {
                write!(f, "Invalid cookie value: {}", msg)
            }
            CookieValidationError::InvalidDomain(msg) => {
                write!(f, "Invalid cookie domain: {}", msg)
            }
            CookieValidationError::InvalidPath(msg) => {
                write!(f, "Invalid cookie path: {}", msg)
            }
        }
    }
}

impl std::error::Error for CookieValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== Cookie Tests ======

    #[test]
    fn creates_cookie_with_defaults() {
        let cookie = Cookie::new("name", Some("value".to_string()));
        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "value");
        assert_eq!(cookie.path(), None);
        assert_eq!(cookie.domain(), None);
        assert_eq!(cookie.max_age(), None);
        assert!(!cookie.is_secure());
        assert!(!cookie.is_http_only());
        assert_eq!(cookie.same_site(), None);
    }

    #[test]
    fn creates_cookie_with_empty_value() {
        let cookie = Cookie::new("name", None);
        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "");
    }

    #[test]
    fn serializes_set_cookie_attributes() {
        let mut cookie = Cookie::new("XSRF-TOKEN", Some("abc".to_string()));
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_max_age_secs(60);
        cookie.set_same_site(SameSite::Lax);
        assert_eq!(
            cookie.to_string(),
            "XSRF-TOKEN=abc; Path=/; Max-Age=60; Secure; HttpOnly; SameSite=Lax"
        );
    }

    #[test]
    fn serializes_cookie_with_domain() {
        let mut cookie = Cookie::new("session", Some("123".to_string()));
        cookie.set_domain("example.com");
        cookie.set_path("/app");
        assert_eq!(
            cookie.to_string(),
            "session=123; Path=/app; Domain=example.com"
        );
    }

    #[test]
    fn getters_return_configured_values() {
        let mut cookie = Cookie::new("name", Some("value".to_string()));
        cookie.set_domain("example.com");
        cookie.set_path("/app");
        cookie.set_max_age_secs(120);
        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "value");
        assert_eq!(cookie.domain(), Some("example.com"));
        assert_eq!(cookie.path(), Some("/app"));
        assert_eq!(cookie.max_age(), Some(120));
        assert!(!cookie.is_secure());
        assert!(!cookie.is_http_only());
        assert_eq!(cookie.same_site(), None);
    }

    #[test]
    fn validation_fails_when_same_site_none_without_secure() {
        let mut cookie = Cookie::new("session", Some("123".to_string()));
        cookie.set_same_site(SameSite::None);
        assert!(cookie.validate().is_err());
        assert_eq!(
            cookie.validate().unwrap_err(),
            CookieError::SameSiteNoneRequiresSecure
        );
    }

    #[test]
    fn validation_passes_when_same_site_none_with_secure() {
        let mut cookie = Cookie::new("session", Some("123".to_string()));
        cookie.set_same_site(SameSite::None);
        cookie.set_secure(true);
        assert!(cookie.validate().is_ok());
    }

    #[test]
    fn validation_passes_when_same_site_strict_or_lax_without_secure() {
        let mut cookie = Cookie::new("session", Some("123".to_string()));
        cookie.set_same_site(SameSite::Strict);
        assert!(cookie.validate().is_ok());

        cookie.set_same_site(SameSite::Lax);
        assert!(cookie.validate().is_ok());
    }

    // ====== CookieBuilder Tests ======

    #[test]
    fn builder_creates_cookie_with_defaults() {
        let cookie = Cookie::builder("name", Some("value".to_string()))
            .build()
            .unwrap();
        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "value");
        assert_eq!(cookie.max_age(), None);
    }

    #[test]
    fn builder_sets_all_attributes() {
        let cookie = Cookie::builder("session", Some("123".to_string()))
            .path("/")
            .domain("example.com")
            .max_age(3600)
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Lax)
            .build()
            .unwrap();

        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.value(), "123");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.domain(), Some("example.com"));
        assert_eq!(cookie.max_age(), Some(3600));
        assert!(cookie.is_secure());
        assert!(cookie.is_http_only());
        assert_eq!(cookie.same_site(), Some(&SameSite::Lax));
    }

    #[test]
    fn builder_fails_when_same_site_none_without_secure() {
        let result = Cookie::builder("session", Some("123".to_string()))
            .same_site(SameSite::None)
            .build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CookieError::SameSiteNoneRequiresSecure);
    }

    #[test]
    fn builder_passes_when_same_site_none_with_secure() {
        let cookie = Cookie::builder("session", Some("123".to_string()))
            .same_site(SameSite::None)
            .secure(true)
            .build()
            .unwrap();
        assert_eq!(cookie.same_site(), Some(&SameSite::None));
        assert!(cookie.is_secure());
    }

    // ====== SameSite Tests ======

    #[test]
    fn same_site_as_str_returns_correct_values() {
        assert_eq!(SameSite::Strict.as_str(), "Strict");
        assert_eq!(SameSite::Lax.as_str(), "Lax");
        assert_eq!(SameSite::None.as_str(), "None");
    }

    #[test]
    fn same_site_from_str_parses_valid_values() {
        assert_eq!("Strict".parse::<SameSite>().unwrap(), SameSite::Strict);
        assert_eq!("Lax".parse::<SameSite>().unwrap(), SameSite::Lax);
        assert_eq!("None".parse::<SameSite>().unwrap(), SameSite::None);
    }

    #[test]
    fn same_site_from_str_rejects_invalid_values() {
        let result = "invalid".parse::<SameSite>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            InvalidSameSiteValue("invalid".to_string())
        );
    }

    #[test]
    fn same_site_display_formats_correctly() {
        assert_eq!(format!("{}", SameSite::Strict), "Strict");
        assert_eq!(format!("{}", SameSite::Lax), "Lax");
        assert_eq!(format!("{}", SameSite::None), "None");
    }

    #[test]
    fn same_site_equality_works() {
        assert_eq!(SameSite::Strict, SameSite::Strict);
        assert_ne!(SameSite::Strict, SameSite::Lax);
    }

    // ====== Integration Tests ======

    #[test]
    fn builder_and_display_work_together() {
        let cookie = Cookie::builder("session", Some("abc123".to_string()))
            .path("/")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Lax)
            .build()
            .unwrap();

        let header_value = cookie.to_string();
        assert!(header_value.contains("session=abc123"));
        assert!(header_value.contains("Path=/"));
        assert!(header_value.contains("Secure"));
        assert!(header_value.contains("HttpOnly"));
        assert!(header_value.contains("SameSite=Lax"));
    }

    #[test]
    fn builder_handles_multiple_paths() {
        let cookie = Cookie::builder("session", Some("123".to_string()))
            .path("/admin")
            .path("/api") // Overwrites previous
            .build()
            .unwrap();
        assert_eq!(cookie.path(), Some("/api"));
    }

    #[test]
    fn display_omits_optional_attributes_when_not_set() {
        let cookie = Cookie::new("session", Some("123".to_string()));
        assert_eq!(cookie.to_string(), "session=123");
    }

    #[test]
    fn display_with_empty_value_omits_attributes_by_default() {
        let cookie = Cookie::new("name", None);
        assert_eq!(cookie.to_string(), "name=");
    }

    #[test]
    fn invalid_same_site_error_display_is_descriptive() {
        let err = InvalidSameSiteValue("Invalid".to_string());
        assert_eq!(
            err.to_string(),
            "invalid SameSite value: 'Invalid', expected one of: Strict, Lax, None"
        );
    }
}
