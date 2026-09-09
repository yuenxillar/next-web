use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use std::sync::Arc;

use crate::{
    core::context::{
        DeferredSecurityContext, SecurityContext, SecurityContextHolder,
        SecurityContextHolderStrategy,
    },
    web::context::{SecurityContextRepository, SuppliedDeferredSecurityContext},
};

/// A [`SecurityContextRepository`] that stores the security context in a request attribute.
#[derive(Clone)]
pub struct RequestAttributeSecurityContextRepository {
    request_attribute_name: String,
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
}

impl RequestAttributeSecurityContextRepository {
    pub const DEFAULT_REQUEST_ATTR_NAME: &str =
        "RequestAttributeSecurityContextRepository.NEXT_SECURITY_CONTEXT";

    /// Creates a new instance with the specified request attribute name.
    pub fn new(request_attribute_name: impl Into<String>) -> Self {
        Self {
            request_attribute_name: request_attribute_name.into(),
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
        }
    }

    pub fn get_context<'a>(
        &'a self,
        request: &'a dyn HttpRequest,
    ) -> Option<&'a Arc<dyn SecurityContext>> {
        request
            .get_attribute(&self.request_attribute_name)
            .and_then(|obj| obj.as_ref_object())
    }

    /// Sets the SecurityContextHolderStrategy to use.
    /// The default action is to use the SecurityContextHolderStrategy stored in SecurityContextHolder.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }
}

impl Default for RequestAttributeSecurityContextRepository {
    fn default() -> Self {
        Self {
            request_attribute_name: Self::DEFAULT_REQUEST_ATTR_NAME.to_string(),
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
        }
    }
}

#[async_trait]
impl SecurityContextRepository for RequestAttributeSecurityContextRepository {
    fn load_deferred_context(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Box<dyn DeferredSecurityContext> {
        Box::new(SuppliedDeferredSecurityContext::new(
            self.get_context(request).cloned(),
            self.security_context_holder_strategy.to_owned(),
        ))
    }

    async fn save_context(
        &self,
        context: &Arc<dyn SecurityContext>,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
        request.set_attribute(
            &self.request_attribute_name,
            AnyValue::Object(Box::new(context.to_owned())),
        );
    }

    fn contains_context(&self, request: &mut dyn HttpRequest) -> bool {
        self.get_context(request).is_some()
    }
}
