use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::trace;

use crate::web::csrf::{
    CsrfToken, CsrfTokenRequestHandler, CsrfTokenRequestResolver, DeferredCsrfToken,
};

/// An implementation of the CsrfTokenRequestHandler interface that is capable of making the
/// CsrfToken available as a request attribute and resolving the token value as either a header or
/// parameter value of the request.
#[derive(Clone)]
pub struct CsrfTokenRequestAttributeHandler {
    csrf_request_attribute_name: Option<Box<str>>,
}

impl CsrfTokenRequestAttributeHandler {
    /// The CsrfToken is available as a request attribute named CsrfToken::type_name().
    /// By default, an additional request attribute that is the same as CsrfToken::parameter_name() is set.
    /// This attribute allows overriding the additional attribute.
    pub fn set_csrf_request_attribute_name(
        &mut self,
        csrf_request_attribute_name: Option<Box<str>>,
    ) {
        self.csrf_request_attribute_name = csrf_request_attribute_name;
    }
}

#[async_trait]
impl CsrfTokenRequestHandler for CsrfTokenRequestAttributeHandler {
    async fn handle(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        deferred_csrf_token: &mut dyn DeferredCsrfToken,
    ) {
        let csrf_token = deferred_csrf_token.token().await;

        let type_name = std::any::type_name::<&dyn CsrfToken>();
        request.set_attribute(type_name, AnyValue::Object(Box::new(csrf_token.clone())));

        let csrf_attr_name = &self
            .csrf_request_attribute_name
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(csrf_token.parameter_name())
            .to_string();

        request.set_attribute(csrf_attr_name, AnyValue::Object(Box::new(csrf_token)));

        trace!(
            "Wrote a CSRF token to the following request attributes: [{}, {}]",
            csrf_attr_name,
            type_name
        );
    }
}

impl CsrfTokenRequestResolver for CsrfTokenRequestAttributeHandler {}

impl Default for CsrfTokenRequestAttributeHandler {
    fn default() -> Self {
        Self {
            csrf_request_attribute_name: Some("_csrf".into()),
        }
    }
}
