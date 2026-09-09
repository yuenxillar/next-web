use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    error::BoxError,
    http::CookieBuilder,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::WebUtils,
};
use uuid::Uuid;

use crate::web::csrf::{CsrfToken, CsrfTokenRepository, DefaultCsrfToken};

/// Default cookie name that the CSRF token is persisted in.
const DEFAULT_CSRF_COOKIE_NAME: &str = "XSRF-TOKEN";
/// Default HTTP request parameter that should contain the token.
const DEFAULT_CSRF_PARAMETER_NAME: &str = "_csrf";
/// Default HTTP header that should contain the token.
const DEFAULT_CSRF_HEADER_NAME: &str = "X-XSRF-TOKEN";

const CSRF_TOKEN_REMOVED_ATTRIBUTE_NAME: &str =
    "next-web-security.web.csrf.CSRF_TOKEN_REMOVED_ATTRIBUTE_NAME.REMOVED";

/// A `CsrfTokenRepository` that persists the CSRF token in a cookie named
/// "XSRF-TOKEN" and reads from the header "X-XSRF-TOKEN" following the
/// conventions of AngularJS. When using with AngularJS be sure to use
/// `with_http_only_false`.
#[derive(Clone)]
pub struct CookieCsrfTokenRepository {
    parameter_name: String,
    header_name: String,
    cookie_name: String,
    cookie_http_only: bool,
    cookie_path: Option<String>,
    cookie_domain: Option<String>,
    secure: Option<bool>,
    cookie_max_age: Option<u32>,
    cookie_customizer: Option<Arc<dyn Fn(&mut CookieBuilder) + Send + Sync>>,
}

impl CookieCsrfTokenRepository {
    /// Factory method to conveniently create an instance that creates cookies
    /// where `http_only` is set to `false`.
    pub fn with_http_only_false() -> Self {
        let mut result = CookieCsrfTokenRepository::default();
        result.cookie_http_only = false;
        result
    }

    /// Add a Consumer for a CookieBuilder that will be invoked for each cookie being built, just before the call to build().
    pub fn set_cookie_customizer(
        &mut self,
        customizer: Arc<dyn Fn(&mut CookieBuilder) + Send + Sync>,
    ) {
        self.cookie_customizer = Some(customizer);
    }

    /// Sets the name of the HTTP request parameter that should be used to
    /// provide a token.
    pub fn set_parameter_name(&mut self, parameter_name: impl Into<String>) {
        let parameter_name = parameter_name.into();
        assert!(!parameter_name.is_empty(), "parameterName cannot be  empty");
        self.parameter_name = parameter_name;
    }

    /// Sets the name of the HTTP header that should be used to provide the
    /// token.
    pub fn set_header_name(&mut self, header_name: impl Into<String>) {
        let header_name = header_name.into();
        assert!(!header_name.is_empty(), "headerName cannot be empty");
        self.header_name = header_name;
    }

    /// Sets the name of the cookie that the expected CSRF token is saved to and
    /// read from.
    pub fn set_cookie_name(&mut self, cookie_name: impl Into<String>) {
        let cookie_name = cookie_name.into();
        assert!(!cookie_name.is_empty(), "cookieName cannot be  empty");
        self.cookie_name = cookie_name;
    }

    /// Sets the path that the cookie will be created with. This overrides the
    /// default behavior which uses the request context as the path.
    pub fn set_cookie_path(&mut self, path: impl Into<String>) {
        self.cookie_path = Some(path.into());
    }

    /// Gets the path that the CSRF cookie will be set to.
    pub fn cookie_path(&self) -> Option<&str> {
        self.cookie_path.as_deref()
    }

    fn get_request_context(request: &dyn HttpRequest) -> String {
        request
            .context_path()
            .filter(|path| !path.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "/".to_string())
    }

    fn create_new_token() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for CookieCsrfTokenRepository {
    fn default() -> Self {
        Self {
            parameter_name: DEFAULT_CSRF_PARAMETER_NAME.to_string(),
            header_name: DEFAULT_CSRF_HEADER_NAME.to_string(),
            cookie_name: DEFAULT_CSRF_COOKIE_NAME.to_string(),
            cookie_http_only: true,
            cookie_path: None,
            cookie_domain: None,
            secure: None,
            cookie_max_age: None,
            cookie_customizer: None,
        }
    }
}

#[async_trait]
impl CsrfTokenRepository for CookieCsrfTokenRepository {
    async fn generate_token(&self, _request: &mut dyn HttpRequest) -> Arc<dyn CsrfToken> {
        Arc::new(DefaultCsrfToken::new(
            self.header_name.as_str(),
            self.parameter_name.as_str(),
            Self::create_new_token(),
        ))
    }

    async fn save_token(
        &self,
        token: Option<&Arc<dyn CsrfToken>>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        let token_value = token.map(|t| t.token()).unwrap_or_default();

        let mut cookie_builder =
            CookieBuilder::new(self.cookie_name.as_str(), Some(token_value.to_string()))
                .secure(self.secure.unwrap_or_else(|| request.is_secure()))
                .path(
                    self.cookie_path
                        .as_deref()
                        .filter(|path| !path.is_empty())
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| Self::get_request_context(request)),
                )
                .max_age(if token.is_some() {
                    self.cookie_max_age
                } else {
                    Some(0)
                })
                .http_only(self.cookie_http_only);
        if let Some(domain) = self.cookie_domain.as_deref().filter(|s| !s.is_empty()) {
            cookie_builder = cookie_builder.domain(domain.to_string());
        }

        self.cookie_customizer
            .as_ref()
            .map(|customizer| customizer(&mut cookie_builder));
        let cookie = cookie_builder.build().map_err(|err| BoxError::from(err))?;

        response.add_cookie(cookie);

        // Set request attribute to signal that response has blank cookie value,
        // which allows load_token to return null when token has been removed
        if token_value.is_empty() {
            request.set_attribute(&CSRF_TOKEN_REMOVED_ATTRIBUTE_NAME, AnyValue::Boolean(true));
        } else {
            request.remove_attribute(&CSRF_TOKEN_REMOVED_ATTRIBUTE_NAME);
        }

        Ok(())
    }

    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>> {
        // Return None when token has been removed during the current request
        // which allows load_deferred_token to re-generate the token
        if request
            .get_attribute(&CSRF_TOKEN_REMOVED_ATTRIBUTE_NAME)
            .and_then(|value| value.as_boolean())
            .unwrap_or_default()
        {
            return None;
        }

        let cookie = WebUtils::get_cookie(request, &self.cookie_name)?;
        let token = cookie.value();
        if token.is_empty() {
            return None;
        }
        Some(Arc::new(DefaultCsrfToken::new(
            self.header_name.as_str(),
            self.parameter_name.as_str(),
            token,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::Request as AxumRequest, response::Response as AxumResponse};

    fn request_with_cookie(cookie_header: &str) -> AxumRequest {
        let mut request = AxumRequest::builder()
            .header("cookie", cookie_header)
            .body(Body::empty())
            .unwrap();
        request.ready();
        request
    }

    #[tokio::test]
    async fn generate_token_uses_default_names() {
        let repo = CookieCsrfTokenRepository::default();
        let mut request = request_with_cookie("");
        let token = repo.generate_token(&mut request).await;
        assert_eq!(token.header_name(), "X-XSRF-TOKEN");
        assert_eq!(token.parameter_name(), "_csrf");
        assert!(!token.token().is_empty());
    }

    #[tokio::test]
    async fn save_token_sets_cookie_and_loads_it() {
        let repo = CookieCsrfTokenRepository::default();
        let mut request = request_with_cookie("");
        let mut response = AxumResponse::new(Body::empty());

        let token = repo.generate_token(&mut request).await;
        repo.save_token(Some(&token), &mut request, &mut response)
            .await
            .inspect_err(|err| {
                eprintln!("Failed to save CSRF token: {}", err);
            })
            .ok();

        let set_cookie = response.header("set-cookie").unwrap_or_default();
        assert!(set_cookie.contains("XSRF-TOKEN="));
        assert!(set_cookie.contains("HttpOnly"));

        let mut request_with_token = request_with_cookie(&format!("XSRF-TOKEN={}", token.token()));
        let loaded = repo
            .load_token(&mut request_with_token)
            .await
            .expect("token should be loaded from cookie");
        assert_eq!(loaded.token(), token.token());
        assert_eq!(loaded.header_name(), "X-XSRF-TOKEN");
    }

    #[tokio::test]
    async fn save_none_token_marks_removed() {
        let repo = CookieCsrfTokenRepository::default();
        let mut request = request_with_cookie("XSRF-TOKEN=abc");
        let mut response = AxumResponse::new(Body::empty());

        repo.save_token(None, &mut request, &mut response)
            .await
            .inspect_err(|err| {
                eprintln!("Failed to save CSRF token: {}", err);
            })
            .ok();

        let set_cookie = response.header("set-cookie").unwrap_or_default();
        assert!(set_cookie.contains("XSRF-TOKEN="));
        assert!(set_cookie.contains("Max-Age=0"));

        assert!(repo.load_token(&mut request).await.is_none());
    }

    #[tokio::test]
    async fn load_token_returns_none_without_cookie() {
        let repo = CookieCsrfTokenRepository::default();
        let mut request = request_with_cookie("other=1");
        assert!(repo.load_token(&mut request).await.is_none());
    }

    #[tokio::test]
    async fn with_http_only_false_disables_http_only() {
        let repo = CookieCsrfTokenRepository::with_http_only_false();
        let mut request = request_with_cookie("");
        let mut response = AxumResponse::new(Body::empty());

        let token = repo.generate_token(&mut request).await;
        repo.save_token(Some(&token), &mut request, &mut response)
            .await
            .inspect_err(|err| {
                eprintln!("Failed to save CSRF token: {}", err);
            })
            .ok();

        let set_cookie = response.header("set-cookie").unwrap_or_default();
        assert!(!set_cookie.contains("HttpOnly"));
    }
}
