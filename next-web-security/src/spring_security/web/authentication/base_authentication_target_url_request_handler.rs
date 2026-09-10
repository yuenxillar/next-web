use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};
use tracing::trace;

use crate::{
    core::Authentication,
    web::{util::UrlUtils, DefaultRedirectStrategy, RedirectStrategy},
};

/// `AuthenticationSuccessHandler` and `LogoutSuccessHandler`, for example.
///
/// Uses the following logic sequence to determine how it should handle the
/// forward/redirect:
///
/// * If the `always_use_default_target_url` property is set to true, the
///   `default_target_url` property will be used for the destination.
/// * If a parameter matching the value of `target_url_parameter` has been set on the
///   request, the value will be used as the destination. If you are enabling this
///   functionality, then you should ensure that the parameter cannot be used by an attacker
///   to redirect the user to a malicious site (by clicking on a URL with the parameter
///   included, for example). Typically it would be used when the parameter is included in
///   the login form and submitted with the username and password.
/// * If the `use_referer` property is set, the "Referer" HTTP header value will be
///   used, if present.
/// * As a fallback option, the `default_target_url` value will be used.
#[derive(Clone)]
pub struct BaseAuthenticationTargetUrlRequestHandler {
    target_url_parameter: Option<Box<str>>,
    default_target_url: Box<str>,
    always_use_default_target_url: bool,
    use_referer: bool,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl BaseAuthenticationTargetUrlRequestHandler {
    /// Invokes the configured `RedirectStrategy` with the URL returned by the
    /// `determine_target_url` method.
    ///
    /// The redirect will not be performed if the response has already been committed.
    pub fn handle(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&dyn Authentication>,
    ) -> Result<(), BoxError> {
        let target_url = self.determine_target_url(request, response, authentication);

        if response.is_committed() {
            tracing::debug!(
                "Did not redirect to {} since response already committed.",
                target_url,
            );
            return Ok(());
        }

        self.redirect_strategy
            .send_redirect(request, response, &target_url)
    }

    /// Builds the target URL
    pub fn determine_target_url<'a>(
        &'a self,
        request: &'a dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _authentication: Option<&dyn Authentication>,
    ) -> &'a str {
        if self.always_use_default_target_url {
            return self.default_target_url.as_ref();
        }

        let target_url_parameter_value = self
            .get_target_url_parameter_value(request)
            .unwrap_or_default();
        if StringUtils::has_text(target_url_parameter_value) {
            trace!(
                "Using url {} from request parameter {:?}",
                target_url_parameter_value,
                self.target_url_parameter.as_ref()
            );
            return target_url_parameter_value.as_ref();
        }

        let referer_header = request.header("referer").unwrap_or_default();
        if !StringUtils::has_text(referer_header) {
            return self.default_target_url.as_ref();
        }

        if self.use_referer {
            trace!("Using url {} from Referer header", referer_header);
            return referer_header;
        }

        self.default_target_url.as_ref()
    }

    fn get_target_url_parameter_value<'a>(
        &'a self,
        request: &'a dyn HttpRequest,
    ) -> Option<&'a str> {
        let target_url_parameter = self.target_url_parameter.as_deref()?;
        let value = request.parameter(target_url_parameter)?;
        if StringUtils::has_text(value) {
            return Some(value);
        } else {
            return Some(target_url_parameter);
        }
    }

    /// Supplies the default target Url that will be used if no saved request is found or the
    /// alwaysUseDefaultTargetUrl property is set to true. If not set, defaults to /
    pub fn get_default_target_url(&self) -> &str {
        &self.default_target_url
    }

    /// Supplies the default target Url that will be used if no saved request is found in
    /// the session, or the `always_use_default_target_url` property is set to true. If
    /// not set, defaults to `/`. It will be treated as relative to the web-app's
    /// context path, and should include the leading `/`. Alternatively,
    /// inclusion of a scheme name (such as "http://" or "https://") as the prefix will
    /// denote a fully-qualified URL and this is also supported.
    pub fn set_default_target_url(&mut self, default_target_url: impl Into<Box<str>>) {
        let default_target_url = default_target_url.into();
        assert!(
            UrlUtils::is_valid_redirect_url(&default_target_url),
            "default_target must start with '/' or with 'http(s)'"
        );
        self.default_target_url = default_target_url;
    }

    /// If true, will always redirect to the value of defaultTargetUrl (defaults to false).
    pub fn set_always_use_default_target_url(&mut self, always_use_default_target_url: bool) {
        self.always_use_default_target_url = always_use_default_target_url;
    }

    pub fn is_always_use_default_target_url(&self) -> bool {
        self.always_use_default_target_url
    }

    /// If this property is set, the current request will be checked for this a
    /// parameter with this name and the value used as the target URL if present.
    pub fn set_target_url_parameter(&mut self, target_url_parameter: impl Into<Box<str>>) {
        let target_url_parameter = target_url_parameter.into();
        assert!(
            StringUtils::has_text(target_url_parameter.as_ref()),
            "target_url_parameter cannot be empty"
        );
        self.target_url_parameter = Some(target_url_parameter);
    }

    pub fn get_target_url_parameter(&self) -> Option<&str> {
        self.target_url_parameter.as_deref()
    }

    /// Allows overriding of the behaviour when redirecting to a target URL.
    pub fn set_redirect_strategy(&mut self, redirect_strategy: Arc<dyn RedirectStrategy>) {
        self.redirect_strategy = redirect_strategy;
    }

    pub fn get_redirect_strategy(&self) -> &dyn RedirectStrategy {
        self.redirect_strategy.as_ref()
    }

    /// If set to true the Referer header will be used (if available). Defaults to false.
    pub fn set_use_referer(&mut self, use_referer: bool) {
        self.use_referer = use_referer;
    }
}

impl Default for BaseAuthenticationTargetUrlRequestHandler {
    fn default() -> Self {
        Self {
            default_target_url: "/".into(),
            target_url_parameter: Default::default(),
            always_use_default_target_url: Default::default(),
            use_referer: Default::default(),
            redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }
}
