use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::HttpMethod,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::web::util::matcher::{PathPatternRequestMatcher, RequestMatcher};

const DEFAULT_CSS: &str = include_str!("resources/default-ui.css");
const DEFAULT_WEBAUTHN_JS: &str = include_str!("resources/spring-security-webauthn.js");

/// Serve common static assets used in default UIs, such as CSS or Javascript
/// files. For internal use only.
///
/// Ported from Spring Security's `DefaultResourcesFilter`.
#[derive(Clone)]
pub struct DefaultResourcesFilter {
    matcher: Arc<dyn RequestMatcher>,
    resource: &'static str,
    media_type: &'static str,
}

impl DefaultResourcesFilter {
    /// Private constructor. Use one of the static factory methods:
    /// [`css`](Self::css) or [`webauthn`](Self::webauthn).
    fn new(
        matcher: Arc<dyn RequestMatcher>,
        resource: &'static str,
        media_type: &'static str,
    ) -> Self {
        Self {
            matcher,
            resource,
            media_type,
        }
    }

    /// Create an instance of `DefaultResourcesFilter` serving Spring Security's
    /// default CSS stylesheet.
    ///
    /// The created filter matches requests `GET /default-ui.css`, and returns
    /// the default stylesheet with content-type `text/css;charset=UTF-8`.
    pub fn css() -> Self {
        Self::new(
            Arc::new(PathPatternRequestMatcher::path_pattern(
                Some(HttpMethod::GET),
                "/default-ui.css",
            )),
            DEFAULT_CSS,
            "text/css;charset=UTF-8",
        )
    }

    /// Create an instance of `DefaultResourcesFilter` serving Spring Security's
    /// default webauthn javascript.
    ///
    /// The created filter matches requests `GET /login/webauthn.js`, and returns
    /// the default webauthn javascript with content-type
    /// `text/javascript;charset=UTF-8`.
    pub fn webauthn() -> Self {
        Self::new(
            Arc::new(PathPatternRequestMatcher::path_pattern(
                Some(HttpMethod::GET),
                "/login/webauthn.js",
            )),
            DEFAULT_WEBAUTHN_JS,
            "text/javascript;charset=UTF-8",
        )
    }
}

impl std::fmt::Display for DefaultResourcesFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultResourcesFilter [matcher={:?}]", self.matcher)
    }
}

#[async_trait]
impl HttpFilter for DefaultResourcesFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if self.matcher.matches(request) {
            response.insert_header("Content-Type", self.media_type);
            response.set_body(self.resource.as_bytes().to_vec());
            return Ok(());
        }

        filter_chain.do_filter(request, response).await?;
        Ok(())
    }
}

impl Named for DefaultResourcesFilter {
    fn name(&self) -> &str {
        "DefaultResourcesFilter"
    }
}
