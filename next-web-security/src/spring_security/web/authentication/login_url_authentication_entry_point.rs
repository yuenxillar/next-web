use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};
use tracing::{debug, warn};

use crate::{
    authorization::RequiredFactorError,
    core::authentication_error::AuthenticationError,
    web::{
        authentication_entry_point::AuthenticationEntryPoint,
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
        util::{RedirectUrlBuilder, UrlUtils},
        PortMapper, PortMapperImpl, WebAttributes,
    },
};

/// Used by the `ExceptionTranslationFilter` to commence a form login authentication
/// via the `UsernamePasswordAuthenticationFilter`.
///
/// Holds the location of the login form in the `login_form_url` property, and uses
/// that to build a redirect URL to the login page. Alternatively, an absolute URL can be
/// set in this property and that will be used exclusively.
///
/// When using a relative URL, you can set the `force_https` property to true, to
/// force the protocol used for the login form to be `HTTPS`, even if the original
/// intercepted request for a resource used the `HTTP` protocol. When this happens,
/// after a successful login (via HTTPS), the original resource will still be accessed as
/// HTTP, via the original request URL. For the forced HTTPS feature to work, the
/// `PortMapper` is consulted to determine the HTTP:HTTPS pairs. The value of
/// `force_https` will have no effect if an absolute URL is used.
#[derive(Clone)]
pub struct LoginUrlAuthenticationEntryPoint {
    port_mapper: Arc<dyn PortMapper>,
    login_form_url: String,
    force_https: bool,
    use_forward: bool,
    favor_relative_uris: bool,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl LoginUrlAuthenticationEntryPoint {
    const FACTOR_PREFIX: &'static str = "FACTOR_";

    /// Creates a new instance with the specified login form URL.
    ///
    /// # Arguments
    /// * `login_form_url` - URL where the login page can be found. Should either be
    ///   relative to the web-app context path (include a leading `/`) or an absolute URL.
    pub fn new(login_form_url: impl Into<String>) -> Self {
        let login_form_url = login_form_url.into();
        assert!(!login_form_url.is_empty(), "login_form_url cannot be empty");
        Self {
            port_mapper: Arc::new(PortMapperImpl::default()),
            login_form_url,
            force_https: false,
            use_forward: false,
            favor_relative_uris: true,
            redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }

    /// Validates the configuration after all properties have been set.
    /// Should be called after dependency injection is complete.
    ///
    /// # Panics
    /// Panics if the configuration is invalid.
    pub fn after_properties_set(&self) {
        assert!(
            StringUtils::has_text(&self.login_form_url)
                && UrlUtils::is_valid_redirect_url(&self.login_form_url),
            "login_form_url must be specified and must be a valid redirect URL"
        );
        assert!(
            !self.use_forward || !UrlUtils::is_absolute_url(&self.login_form_url),
            "use_forward must be false if using an absolute login_form_url"
        );
    }

    /// Allows subclasses to modify the login form URL that should be applicable for a
    /// given request.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    /// * `response` - the HTTP response
    /// * `error` - the authentication error
    ///
    /// # Returns
    /// The URL to use for this request (defaults to `get_login_form_url()`)
    pub fn determine_url_to_use_for_this_request(
        &self,
        request: &dyn HttpRequest,
        _response: &dyn HttpResponse,
        _error: &AuthenticationError,
    ) -> String {
        let factor_errors = self.get_attribute::<Vec<RequiredFactorError>>(
            request,
            WebAttributes::REQUIRED_FACTOR_ERRORS,
        );

        if factor_errors.map(|val| val.is_empty()).unwrap_or(true) {
            return self.get_login_form_url().to_string();
        }

        let factor_errors = factor_errors.expect("Required factor errors attribute not found");

        let factor_types: Vec<String> = factor_errors
            .iter()
            .map(|error| {
                let authority = error.required_factor().authority();
                authority[Self::FACTOR_PREFIX.len()..].to_lowercase()
            })
            .collect();

        let factor_reasons: Vec<String> = factor_errors
            .iter()
            .map(|error| {
                if error.is_expired() {
                    "expired"
                } else {
                    "missing"
                }
                .to_string()
            })
            .collect();

        build_uri_with_query_params(
            self.get_login_form_url(),
            &[
                ("factor.type", &factor_types),
                ("factor.reason", &factor_reasons),
            ],
        )
    }

    fn get_attribute<'a, T>(&'a self, request: &'a dyn HttpRequest, name: &str) -> Option<&'a T>
    where
        T: 'static,
    {
        request
            .get_attribute(name)
            .and_then(|val| val.as_ref_object::<T>())
    }

    /// Builds the redirect URL to the login page, considering force HTTPS and
    /// absolute/relative URL settings.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    /// * `response` - the HTTP response
    /// * `auth_error` - the authentication exception
    ///
    /// # Returns
    /// The redirect URL to the login page
    pub fn build_redirect_url_to_login_page(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> String {
        let login_form = self.determine_url_to_use_for_this_request(request, response, auth_error);

        if UrlUtils::is_absolute_url(&login_form) {
            return login_form;
        }

        if self.requires_rewrite(request) {
            return self.https_uri(request, &login_form);
        }

        if self.favor_relative_uris {
            login_form
        } else {
            self.absolute_uri(request, &login_form).get_url()
        }
    }

    /// Checks whether the request needs to be rewritten from HTTP to HTTPS.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    ///
    /// # Returns
    /// `true` if HTTPS should be forced and the request uses HTTP
    fn requires_rewrite(&self, request: &dyn HttpRequest) -> bool {
        self.force_https && request.scheme() == Some("http")
    }

    /// Builds an HTTPS URI for the given path by looking up the HTTPS port mapping.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    /// * `path` - the path to redirect to
    ///
    /// # Returns
    /// The HTTPS URL
    fn https_uri(&self, request: &dyn HttpRequest, path: &str) -> String {
        let server_port = self.get_server_port(request);
        let https_port = self.port_mapper.lookup_https_port(server_port);

        let https_port = match https_port {
            Some(port) => port,
            None => {
                warn!(
                    "Unable to redirect to HTTPS as no port mapping found for HTTP port {}",
                    server_port
                );

                return if self.favor_relative_uris {
                    path.to_string()
                } else {
                    self.absolute_uri(request, path).get_url()
                };
            }
        };

        let mut builder = self.absolute_uri(request, path);
        builder.set_scheme("https");
        builder.set_port(https_port);
        builder.get_url()
    }

    /// Builds an absolute URI for the given path using the current request information.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    /// * `path` - the path
    ///
    /// # Returns
    /// A `RedirectUrlBuilder` configured with the absolute URI
    fn absolute_uri(&self, request: &dyn HttpRequest, path: &str) -> RedirectUrlBuilder {
        let mut url_builder = RedirectUrlBuilder::default();
        request.scheme().map(|s| url_builder.set_scheme(s));
        request
            .server_name()
            .map(|s| url_builder.set_server_name(s));
        url_builder.set_port(self.get_server_port(request));
        request
            .context_path()
            .map(|s| url_builder.set_context_path(s));
        url_builder.set_path(path);

        url_builder
    }

    /// Builds a URL to redirect the supplied request to HTTPS. Used to redirect the
    /// current request to HTTPS, before doing a forward to the login page.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    ///
    /// # Returns
    /// The HTTPS redirect URL, or `None` if no port mapping is found
    ///
    /// # Errors
    /// Returns an error if URL building fails
    pub fn build_https_redirect_url_for_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<String> {
        let server_port = self.get_server_port(request);
        let https_port = self.port_mapper.lookup_https_port(server_port);

        if let Some(port) = https_port {
            let mut url_builder = RedirectUrlBuilder::default();
            url_builder.set_scheme("https");
            request
                .server_name()
                .map(|s| url_builder.set_server_name(s));

            url_builder.set_port(port);
            request
                .context_path()
                .map(|s| url_builder.set_context_path(s));
            url_builder.set_path(request.path());
            request.query().map(|s| url_builder.set_query(s));

            return Some(url_builder.get_url());
        }

        // Fall through to server-side forward with warning message
        warn!(
            "Unable to redirect to HTTPS as no port mapping found for HTTP port {}",
            server_port
        );

        None
    }

    /// Gets the server port from the request, using the port mapper for resolution.
    ///
    /// # Arguments
    /// * `request` - the servlet request
    ///
    /// # Returns
    /// The server port
    pub fn get_server_port(&self, request: &dyn HttpRequest) -> u16 {
        self.port_mapper.get_server_port(request)
    }

    /// Set to true to force login form access to be via https. If this value is true (the
    /// default is false), and the incoming request for the protected resource which
    /// triggered the interceptor was not already `https`, then the client will first be
    /// redirected to an https URL, even if `server_side_redirect` is set to `true`.
    ///
    /// # Arguments
    /// * `force_https` - whether to force HTTPS
    pub fn set_force_https(&mut self, force_https: bool) {
        self.force_https = force_https;
    }

    /// Returns whether HTTPS is forced for login form access.
    ///
    /// # Returns
    /// `true` if HTTPS is forced, `false` otherwise
    pub fn is_force_https(&self) -> bool {
        self.force_https
    }

    /// Returns the configured login form URL.
    ///
    /// # Returns
    /// The login form URL
    pub fn get_login_form_url(&self) -> &str {
        &self.login_form_url
    }

    /// Sets the port mapper used to determine HTTP:HTTPS port pairs.
    ///
    /// # Arguments
    /// * `port_mapper` - the port mapper to use
    pub fn set_port_mapper(&mut self, port_mapper: Arc<dyn PortMapper>) {
        self.port_mapper = port_mapper;
    }

    /// Returns a reference to the current port mapper.
    ///
    /// # Returns
    /// Reference to the `PortMapper` implementation
    pub fn get_port_mapper(&self) -> &dyn PortMapper {
        self.port_mapper.as_ref()
    }

    /// Tells if we are to do a forward to the `login_form_url` using the
    /// `RequestDispatcher`, instead of a 302 redirect.
    ///
    /// # Arguments
    /// * `use_forward` - true if a forward to the login page should be used. Must be false
    ///   (the default) if `login_form_url` is set to an absolute value.
    pub fn set_use_forward(&mut self, use_forward: bool) {
        self.use_forward = use_forward;
    }

    /// Returns whether forward is used instead of redirect.
    ///
    /// # Returns
    /// `true` if forward is used, `false` if redirect is used
    pub fn is_use_forward(&self) -> bool {
        self.use_forward
    }

    /// Favor using relative URIs when formulating a redirect.
    ///
    /// Note that a relative redirect is not always possible. For example, when redirecting
    /// from `http` to `https`, the URL needs to be absolute.
    ///
    /// # Arguments
    /// * `favor_relative_uris` - whether to favor relative URIs or not
    pub fn set_favor_relative_uris(&mut self, favor_relative_uris: bool) {
        self.favor_relative_uris = favor_relative_uris;
    }
}

impl AuthenticationEntryPoint for LoginUrlAuthenticationEntryPoint {
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        if !self.use_forward {
            // redirect to login page. Use https if forceHttps true
            let redirect_url = self.build_redirect_url_to_login_page(request, response, auth_error);
            self.redirect_strategy
                .send_redirect(request, response, &redirect_url)?;
            return Ok(());
        }

        let redirect_url = if self.requires_rewrite(request) {
            // First redirect the current request to HTTPS. When that request is received,
            // the forward to the login page will be used.
            self.build_https_redirect_url_for_request(request)
        } else {
            None
        };

        if let Some(url) = redirect_url.as_deref() {
            self.redirect_strategy
                .send_redirect(request, response, url)?;
            return Ok(());
        }

        let login_form = self.determine_url_to_use_for_this_request(request, response, auth_error);
        debug!("Server side forward to: {}", login_form);

        if let Some(request_dispatcher) = request.request_dispatcher(&login_form) {
            request_dispatcher.forward(request, response)?;
        }

        Ok(())
    }
}

/// Builds a URI with query parameters for multi-valued parameters.
///
/// # Arguments
/// * `base_url` - the base URL
/// * `params` - slice of (parameter name, values) pairs
///
/// # Returns
/// The URI with appended query parameters
fn build_uri_with_query_params(base_url: &str, params: &[(&str, &[String])]) -> String {
    let mut result = base_url.to_string();
    let mut first = !base_url.contains('?');

    for (name, values) in params {
        for value in *values {
            if first {
                result.push('?');
                first = false;
            } else {
                result.push('&');
            }
            result.push_str(&format!("{}={}", name, value));
        }
    }

    result
}
