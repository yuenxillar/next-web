use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};
use tracing::trace;

use crate::{
    core::Authentication,
    web::redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
};

/// Base class containing the logic used by strategies which handle redirection to a URL and are passed an
/// Authentication object as part of the contract. See AuthenticationSuccessHandler and LogoutSuccessHandler,
#[derive(Clone)]
pub struct BaseAuthenticationTargetUrlRequestHandler {
    target_url_parameter: Option<Box<str>>,
    default_target_url: Box<str>,
    always_use_default_target_url: bool,
    use_referer: bool,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl BaseAuthenticationTargetUrlRequestHandler {
    /// Invokes the configured RedirectStrategy with the URL returned by the determineTargetUrl method.
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
    pub fn determine_target_url(
        &self,
        request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _authentication: Option<&dyn Authentication>,
    ) -> String {
        if self.always_use_default_target_url {
            return self.default_target_url.to_string();
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
            return target_url_parameter_value.to_string();
        }

        let referer_header = request.header("referer").unwrap_or_default();
        if !StringUtils::has_text(referer_header) {
            return self.default_target_url.to_string();
        }

        if self.use_referer {
            trace!("Using url {} from Referer header", referer_header);
            return referer_header.to_string();
        }

        self.default_target_url.to_string()
    }

    pub fn set_default_target_url(&mut self, default_target_url: impl Into<Box<str>>) {
        let default_target_url = default_target_url.into();
        assert!(
            !default_target_url.trim().is_empty(),
            "default_target_url cannot be empty"
        );
        self.default_target_url = default_target_url;
    }

    fn get_target_url_parameter_value<'a>(
        &'a self,
        request: &'a dyn HttpRequest,
    ) -> Option<&'a str> {
        match self.target_url_parameter.as_deref() {
            Some(value) => {
                let s = request.parameter(value)?;
                if StringUtils::has_text(s) {
                    return Some(s);
                } else {
                    return Some(value);
                }
            }
            None => return None,
        }
    }

    /// Supplies the default target Url that will be used if no saved request is found or the
    /// alwaysUseDefaultTargetUrl property is set to true. If not set, defaults to /
    pub fn get_default_target_url(&self) -> &str {
        &self.default_target_url
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
    pub fn set_redirect_strategy<T>(&mut self, redirect_strategy: T)
    where
        T: RedirectStrategy,
        T: 'static,
    {
        self.redirect_strategy = Arc::new(redirect_strategy);
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
