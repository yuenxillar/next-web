//! The filter that writes the authorities a rule reads into the request.

use std::sync::Arc;

use next_web_core::{
    any::AnyValue,
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    security::{PreAuthorizeDenialHandler, AUTHORITIES_ATTRIBUTE, DENIAL_HANDLER_ATTRIBUTE},
    Named,
};

use crate::{
    core::context::SecurityContextHolder,
    web::access::{prepost::chain_denial_handler::ChainDenialHandler, AccessDeniedHandler},
};

/// Writes the authorities of the request, and the handler of a refusal, into
/// the request.
///
/// A rule a handler is guarded with is evaluated while the handler runs, which
/// is after the filter chain of the request finished, and the authentication of
/// a request is held by the thread that runs the filters. The filter therefore
/// writes the authorities of the authentication the chain established, and the
/// handler that renders a refusal, into the request itself, so the rule of the
/// handler reads the state of the request it is handled with and not the state
/// of the thread that happens to run it.
#[derive(Clone)]
pub struct PreAuthorizeAuthoritiesFilter {
    denial_handler: Arc<dyn PreAuthorizeDenialHandler>,
}

impl PreAuthorizeAuthoritiesFilter {
    /// Creates the filter.
    ///
    /// # Arguments
    ///
    /// * `access_denied_handler` - The handler of a refused request that the
    ///   application configured for its filter chain.
    pub fn new(access_denied_handler: Arc<dyn AccessDeniedHandler>) -> Self {
        Self {
            denial_handler: Arc::new(ChainDenialHandler::new(access_denied_handler)),
        }
    }

    /// Returns the authorities of the authentication of the request.
    ///
    /// A request without an authentication answers with no authority, so a rule
    /// that asks for a role or an authority of it does not hold.
    fn authorities() -> Vec<AnyValue> {
        SecurityContextHolder::get_context()
            .get_authentication()
            .map(|authentication| {
                authentication
                    .authorities()
                    .iter()
                    .filter_map(|authority| authority.authority().map(AnyValue::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl HttpFilter for PreAuthorizeAuthoritiesFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // The authorities are read once the filters of the request ran: the
        // filters that authenticate the request are the ones that established
        // the authentication the rules are evaluated against.
        let result = filter_chain.do_filter(request, response).await;

        request.set_attribute(AUTHORITIES_ATTRIBUTE, AnyValue::Vec(Self::authorities()));
        request.set_attribute(
            DENIAL_HANDLER_ATTRIBUTE,
            AnyValue::BoxedValue(Box::new(self.denial_handler.clone())),
        );

        result
    }
}

impl Named for PreAuthorizeAuthoritiesFilter {
    fn name(&self) -> &str {
        "PreAuthorizeAuthoritiesFilter"
    }
}
