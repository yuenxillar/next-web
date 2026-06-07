use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;
use tracing::trace;

use crate::web::csrf::{CsrfToken, CsrfTokenRequestHandler, CsrfTokenRequestResolver};

pub struct CsrfTokenRequestAttributeHandler {
    csrf_request_attribute_name: Option<Box<str>>,
}

impl CsrfTokenRequestAttributeHandler {
    pub fn set_csrf_request_attribute_name(
        &mut self,
        csrf_request_attribute_name: Option<Box<str>>,
    ) {
        self.csrf_request_attribute_name = csrf_request_attribute_name;
    }
}

impl CsrfTokenRequestHandler for CsrfTokenRequestAttributeHandler {
    fn handle(
        &self,
        request: &mut dyn next_web_core::traits::http::http_request::HttpRequest,
        _response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
        deferred_csrf_token: &dyn Fn() -> Arc<dyn CsrfToken>,
    ) {
        let csrf_token = deferred_csrf_token();

        let type_name = std::any::type_name::<&dyn CsrfToken>();
        request.set_attribute(type_name, AnyValue::Object(Box::new(csrf_token.clone())));

        let csrf_attr_name = self
            .csrf_request_attribute_name
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(csrf_token.get_parameter_name());

        request.set_attribute(
            csrf_attr_name,
            AnyValue::Object(Box::new(csrf_token.clone())),
        );

        trace!(
            "Wrote a CSRF token to the following request attributes: [{}, {}]",
            csrf_attr_name,
            type_name
        );
    }
}

impl CsrfTokenRequestResolver for CsrfTokenRequestAttributeHandler {}

struct SupplierCsrfToken {
    csrf_token_supplier: Arc<dyn Fn() -> Arc<dyn CsrfToken>>,
}

impl SupplierCsrfToken {
    fn new(csrf_token_supplier: Arc<dyn Fn() -> Arc<dyn CsrfToken>>) -> Self {
        Self {
            csrf_token_supplier,
        }
    }

    fn get_delegate(&self) -> Arc<dyn CsrfToken> {
        (self.csrf_token_supplier)()
    }
}

impl Default for CsrfTokenRequestAttributeHandler {
    fn default() -> Self {
        Self {
            csrf_request_attribute_name: Some("_csrf".into()),
        }
    }
}
