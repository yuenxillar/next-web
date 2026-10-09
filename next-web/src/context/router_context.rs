#[cfg(feature = "enable-open-api")]
use next_web_open_api::OpenApiRouter;

/// Shared state passed to every [`HttpHandlerAutoRegister::register`] call.
///
/// It currently only carries OpenAPI-related state, and the field is
/// conditionally compiled so that builds without the `enable-open-api`
/// feature do not pay for the dependency or the extra field.
pub struct RouterContext {
    /// Accumulated OpenAPI specification, if the `enable-open-api`
    /// feature is enabled.
    ///
    /// It is `None` until [`RouterContext::with_openapi`] is called, and
    /// is later taken out by the router setup once all handlers have
    /// registered their paths.
    #[cfg(feature = "enable-open-api")]
    pub open_api: Option<OpenApiRouter>,
}

impl RouterContext {
    /// Returns an empty context.
    ///
    /// When the `enable-open-api` feature is enabled, `open_api` starts
    /// as `None`; callers that need OpenAPI support should use
    /// [`RouterContext::with_openapi`] instead.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "enable-open-api")]
            open_api: None,
        }
    }

    /// Creates a [`RouterContext`] pre-populated with an OpenAPI spec.
    ///
    /// The spec is wrapped in an [`OpenApiRouter`] so that handlers can
    /// register their paths and schemas into it as they are visited.
    ///
    /// Only available when the `enable-open-api` feature is enabled.
    #[cfg(feature = "enable-open-api")]
    pub fn with_openapi(openapi: next_web_open_api::openapi::OpenApi) -> Self {
        let mut ctx = Self::default();
        ctx.open_api = Some(OpenApiRouter::with_openapi(openapi));
        ctx
    }
}

impl Default for RouterContext {
    fn default() -> Self {
        Self::new()
    }
}
