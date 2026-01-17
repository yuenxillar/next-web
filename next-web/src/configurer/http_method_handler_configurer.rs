#[cfg(feature = "enable-api-doc")]
use next_web_api_doc::OpenApiRouter;

pub struct HttpMethodHandlerConfigurer;

impl Default for HttpMethodHandlerConfigurer {
    fn default() -> Self {
        Self {}
    }
}

pub struct RouterContext {
    pub(crate) state: RouteState,
    pub(crate) index: usize,

    #[cfg(feature = "enable-api-doc")]
    pub open_api: Option<OpenApiRouter>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RouteState {
    #[default]
    None,

    Default,
    /// This means the polling is over
    End,
}

impl RouterContext {
    #[cfg(feature = "enable-api-doc")]
    pub fn with_openapi(openapi: utoipa::openapi::OpenApi) -> Self {
        let mut ctx = Self::default();
        ctx.open_api = Some(OpenApiRouter::with_openapi(openapi));
        ctx
    }
}

impl Iterator for RouterContext {
    type Item = RouteState;

    fn next(&mut self) -> Option<Self::Item> {
        let elements = [RouteState::Default, RouteState::End];

        if self.index >= elements.len() {
            return None;
        }

        self.state = elements[self.index];

        let result = Some(elements[self.index]);
        self.index += 1;

        result
    }
}

impl Default for RouterContext {
    fn default() -> Self {
        Self {
            state: RouteState::None,
            index: 0,
            #[cfg(feature = "enable-api-doc")]
            open_api: None,
        }
    }
}
