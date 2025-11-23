pub mod proxied_filter_chain;
use std::fmt::Write;

use chrono::Utc;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};
use tracing::{debug, warn};

pub mod mgt;
// pub mod web_security_context;
pub mod filter;
pub mod filter_proxy;
pub mod session;
pub mod subject;

pub trait Cookie: Send + Sync {
    fn get_name(&self) -> Option<&str>;

    fn set_name(&mut self, name: String);

    fn get_value(&self) -> Option<&str>;

    fn set_value(&mut self, value: String);

    fn get_comment(&self) -> Option<&str>;

    fn set_comment(&mut self, comment: String);

    fn get_domain(&self) -> Option<&str>;

    fn set_domain(&mut self, domain: String);

    fn get_max_age(&self) -> i32;

    fn set_max_age(&mut self, max_age: i32);

    fn get_path(&self) -> Option<&str>;

    fn set_path(&mut self, path: String);

    fn is_secure(&self) -> bool;

    fn set_secure(&mut self, secure: bool);

    fn get_version(&self) -> i32;

    fn set_version(&mut self, version: i32);

    fn is_http_only(&self) -> bool;

    fn set_http_only(&mut self, http_only: bool);

    fn set_same_site(&mut self, same_site: SameSite);

    fn get_same_site(&self) -> Option<&SameSite>;

    fn save_to(&self, req: Option<&dyn HttpRequest>, resp: &mut dyn HttpResponse);

    fn remove_from(&self, req: &dyn HttpRequest, resp: &mut dyn HttpResponse);

    fn read_value(&self, req: &dyn HttpRequest, resp: &dyn HttpResponse) -> Option<String>;
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    #[default]
    Lax,
    None,
}

impl AsRef<str> for SameSite {
    fn as_ref(&self) -> &str {
        match self {
            SameSite::Strict => "strict",
            SameSite::Lax => "lax",
            SameSite::None => "none",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimpleCookie {
    name: Option<String>,
    value: Option<String>,
    comment: Option<String>,
    domain: Option<String>,
    path: Option<String>,
    max_age: Option<i32>, // seconds; None = session cookie
    version: Option<i32>,
    secure: bool,
    http_only: bool,
    same_site: Option<SameSite>,
}

impl SimpleCookie {
    const DEFAULT_MAX_AGE: i32 = -1;
    const DEFAULT_VERSION: i32 = -1;
    const NAME_VALUE_DELIMITER: &str = "=";
    const ATTRIBUTE_DELIMITER: &str = "; ";
    const DAY_MILLIS: i64 = 86400000;
    const GMT_TIME_ZONE_ID: &str = "GMT";
    const COOKIE_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
    const COOKIE_HEADER_NAME: &str = "Set-Cookie";
    const PATH_ATTRIBUTE_NAME: &str = "Path";
    const EXPIRES_ATTRIBUTE_NAME: &str = "Expires";
    const MAXAGE_ATTRIBUTE_NAME: &str = "Max-Age";
    const DOMAIN_ATTRIBUTE_NAME: &str = "Domain";
    const VERSION_ATTRIBUTE_NAME: &str = "Version";
    const COMMENT_ATTRIBUTE_NAME: &str = "Comment";
    const SECURE_ATTRIBUTE_NAME: &str = "Secure";
    const HTTP_ONLY_ATTRIBUTE_NAME: &str = "HttpOnly";
    const SAME_SITE_ATTRIBUTE_NAME: &str = "SameSite";
    const ROOT_PATH: &'static str = "/";
    pub const DELETED_COOKIE_VALUE: &'static str = "deleteMe";
    pub const ONE_YEAR: i32 = 60 * 60 * 24 * 365;
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        if name.is_empty() {
            panic!("Cookie name cannot be empty");
        }
        let mut cookie = Self::default();
        cookie.name = Some(name);
        cookie
    }

    pub fn clear_same_site(&mut self) {
        self.same_site = None;
    }

    /// Check if cookie path matches request path (per RFC 6265 Section 5.1.4)
    pub fn path_matches(&self, cookie_path: &str, request_path: &str) -> bool {
        if !request_path.starts_with(cookie_path) {
            return false;
        }

        request_path.len() == cookie_path.len()
            || &cookie_path[cookie_path.len() - 1..] == "/"
            || request_path[cookie_path.len()..].starts_with("/")
    }

    fn add_cookie_header(
        &self,
        response: &mut dyn HttpResponse,
        name: &str,
        value: Option<&str>,
        comment: Option<&str>,
        domain: &str,
        path: &str,
        max_age: i32,
        version: i32,
        secure: bool,
        http_only: bool,
        same_site: Option<&SameSite>,
    ) {
        let header_value = self.build_header_value(
            name, value, comment, domain, path, max_age, version, secure, http_only, same_site,
        );
        response.append_header(Self::COOKIE_HEADER_NAME.as_bytes(), &header_value);

        debug!("Adding cookie header_value: {}", header_value);
    }

    pub fn build_header_value(
        &self,
        name: &str,
        value: Option<&str>,
        comment: Option<&str>,
        domain: &str,
        path: &str,
        max_age: i32,
        version: i32,
        secure: bool,
        http_only: bool,
        same_site: Option<&SameSite>,
    ) -> String {
        if name.is_empty() {
            warn!("Cookie name cannot be empty. Skipping cookie.");
            return String::new();
        }
        let mut cookie = format!("{}{}", name, Self::NAME_VALUE_DELIMITER);
        if let Some(value) = value {
            cookie.push_str(value);
        }

        if let Some(comment) = comment {
            Self::append(&mut cookie, Self::COMMENT_ATTRIBUTE_NAME, comment);
        }

        Self::append(&mut cookie, Self::DOMAIN_ATTRIBUTE_NAME, domain);
        Self::append(&mut cookie, Self::PATH_ATTRIBUTE_NAME, path);
        Self::append_expires(&mut cookie, max_age);
        Self::append(
            &mut cookie,
            Self::VERSION_ATTRIBUTE_NAME,
            &version.to_string(),
        );
        Self::append_secure(&mut cookie, secure);
        Self::append_http_only(&mut cookie, http_only);
        Self::append_same_site(&mut cookie, same_site);
        cookie
    }

    fn append(buffer: &mut String, name: &str, value: &str) {
        if !value.is_empty() {
            write!(
                buffer,
                "{}{}{}{}",
                Self::ATTRIBUTE_DELIMITER,
                name,
                Self::NAME_VALUE_DELIMITER,
                value
            )
            .ok();
        }
    }

    fn append_expires(buffer: &mut String, max_age: i32) {
        if max_age > 0 {
            write!(
                buffer,
                "{}{}{}{}{}",
                Self::ATTRIBUTE_DELIMITER,
                Self::MAXAGE_ATTRIBUTE_NAME,
                Self::NAME_VALUE_DELIMITER,
                max_age,
                Self::ATTRIBUTE_DELIMITER
            ).unwrap();

            let expires = if max_age == 0 {
                Utc::now() - chrono::Duration::milliseconds(Self::DAY_MILLIS)
            } else {
                Utc::now() + chrono::Duration::seconds(max_age as i64)
            };

            write!(
                buffer,
                "{}{}{}",
                Self::EXPIRES_ATTRIBUTE_NAME,
                Self::NAME_VALUE_DELIMITER,
                expires.format(Self::COOKIE_DATE_FORMAT).to_string()
            ).unwrap();
        }
    }

    fn append_secure(buffer: &mut String, secure: bool) {
        if secure {
            write!(
                buffer,
                "{}{}",
                Self::ATTRIBUTE_DELIMITER,
                Self::SECURE_ATTRIBUTE_NAME
            )
            .ok();
        }
    }

    fn append_http_only(buffer: &mut String, http_only: bool) {
        if http_only {
            write!(
                buffer,
                "{}{}",
                Self::ATTRIBUTE_DELIMITER,
                Self::HTTP_ONLY_ATTRIBUTE_NAME
            )
            .ok();
        }
    }

    fn append_same_site(buffer: &mut String, same_site: Option<&SameSite>) {
        if let Some(same_site) = same_site {
            write!(
                buffer,
                "{}{}{}{}",
                Self::ATTRIBUTE_DELIMITER,
                Self::SAME_SITE_ATTRIBUTE_NAME,
                Self::NAME_VALUE_DELIMITER,
                same_site.as_ref()
            )
            .ok();
        }
    }

    pub fn calculate_path<'a>(&'a self, req: &'a dyn HttpRequest) -> &'a str {
        let path = self
            .get_path()
            .map(|s| s.trim())
            .map(|s| {
                if s.is_empty() {
                    req.context_path().trim()
                } else {
                    s
                }
            })
            .unwrap_or(Self::ROOT_PATH);

        path
    }
}

impl Cookie for SimpleCookie {
    fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: String) {
        assert!(!name.is_empty(), "name cannot be empty.");

        self.name = Some(name);
    }

    fn get_value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    fn set_value(&mut self, value: String) {
        self.value = Some(value);
    }

    fn get_comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    fn set_comment(&mut self, comment: String) {
        self.comment = Some(comment);
    }

    fn get_domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    fn set_domain(&mut self, domain: String) {
        self.domain = Some(domain);
    }

    fn get_max_age(&self) -> i32 {
        self.max_age.unwrap_or_default()
    }

    fn set_max_age(&mut self, max_age: i32) {
        self.max_age = Some((Self::DEFAULT_MAX_AGE).max(max_age));
    }

    fn get_path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn set_path(&mut self, path: String) {
        self.path = Some(path);
    }

    fn is_secure(&self) -> bool {
        self.secure
    }

    fn set_secure(&mut self, secure: bool) {
        self.secure = secure;
    }

    fn get_version(&self) -> i32 {
        self.version.unwrap_or_default()
    }

    fn set_version(&mut self, version: i32) {
        self.version = Some(Self::DEFAULT_VERSION.max(version));
    }

    fn is_http_only(&self) -> bool {
        self.http_only
    }

    fn set_http_only(&mut self, http_only: bool) {
        self.http_only = http_only;
    }

    fn set_same_site(&mut self, same_site: SameSite) {
        if same_site == SameSite::None {
            self.secure = true;
        }
        self.same_site = Some(same_site);
    }

    fn get_same_site(&self) -> Option<&SameSite> {
        self.same_site.as_ref()
    }

    fn save_to(&self, _req: Option<&dyn HttpRequest>, resp: &mut dyn HttpResponse) {
        self.add_cookie_header(
            resp,
            self.get_name().unwrap_or_default(),
            self.get_value(),
            self.get_comment(),
            self.get_domain().unwrap_or_default(),
            self.get_path().unwrap_or_default(),
            self.get_max_age(),
            self.get_version(),
            self.is_secure(),
            self.is_http_only(),
            self.get_same_site(),
        );
    }

    fn remove_from(&self, req: &dyn HttpRequest, resp: &mut dyn HttpResponse) {
        self.add_cookie_header(
            resp,
            self.get_name().unwrap_or_default(),
            Some(Self::DELETED_COOKIE_VALUE),
            None,
            self.get_domain().unwrap_or_default(),
            self.calculate_path(req),
            0,
            self.get_version(),
            self.is_secure(),
            false,
            self.get_same_site(),
        );

        debug!(
            "Removing {:?} cookie by setting max_age = 0",
            self.get_name()
        );
    }

    fn read_value(&self, req: &dyn HttpRequest, _resp: &dyn HttpResponse) -> Option<String> {
        let name = self.get_name().unwrap_or_default();

        let cookie = req.cookie();
        if let Some(cookie) = cookie {
            if !cookie
                .get("name")
                .map(|s| self.get_name().map(|s1| s1.eq(s)).unwrap_or_default())
                .unwrap_or_default()
            {
                return None;
            }

            let path = self.get_path().map(str::trim).unwrap_or_default();
            if !path.is_empty() && self.path_matches(path, req.path()) {
                warn!(
                    "Found {} cookie at path {}, but should be only used for {}",
                    name,
                    req.path(),
                    path
                );
            } else {
                let value = cookie.get("value");
                debug!("Found {} cookie value {:?}", name, value);
                return value.map(ToString::to_string);
            }
        } else {
            debug!("No {} cookie value", name);
        }

        None
    }
}

impl From<&dyn Cookie> for SimpleCookie {
    fn from(cookie: &dyn Cookie) -> Self {
        Self {
            name: cookie.get_name().map(ToString::to_string),
            value: cookie.get_value().map(ToString::to_string),
            comment: cookie.get_comment().map(ToString::to_string),
            domain: cookie.get_domain().map(ToString::to_string),
            path: cookie.get_path().map(ToString::to_string),
            max_age: Some(cookie.get_max_age().max(Self::DEFAULT_MAX_AGE)),
            version: Some(cookie.get_version().max(Self::DEFAULT_VERSION)),
            secure: cookie.is_secure(),
            http_only: cookie.is_http_only(),
            same_site: cookie.get_same_site().map(Clone::clone),
        }
    }
}
impl Default for SimpleCookie {
    fn default() -> Self {
        Self {
            name: Default::default(),
            value: Default::default(),
            comment: Default::default(),
            domain: Default::default(),
            path: Default::default(),
            max_age: Some(Self::DEFAULT_MAX_AGE),
            version: Some(Self::DEFAULT_VERSION),
            http_only: true,
            same_site: Some(Default::default()),
            secure: Default::default(),
        }
    }
}
