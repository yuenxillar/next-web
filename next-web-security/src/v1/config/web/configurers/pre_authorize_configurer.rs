//! Configures the state the access-control rules of a handler read.

use std::{ops::Deref, sync::Arc};

use next_web_core::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::{
        access::{
            prepost::PreAuthorizeAuthoritiesFilter, AccessDeniedHandler, AccessDeniedHandlerImpl,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

use super::ErrorHandlingConfigurer;

/// Adds the filter that writes the authorities of a request, and the handler of
/// a refusal, into the request.
///
/// The rules a handler is guarded with are evaluated while the handler runs,
/// which is after the filter chain of the request finished. The filter this
/// configurer adds writes the authorities of the authentication the chain
/// established into the request, so the rule of a handler reads them from the
/// request it is handled with.
#[derive(Clone)]
pub struct PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    base: BaseHttpConfigurer<PreAuthorizeConfigurer<H>, H>,
}

impl<H> PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H> + 'static,
{
    /// Returns the handler the refusal of a rule is rendered with.
    ///
    /// The handler of the application is used when it configured one for its
    /// filter chain, and the default handler is used otherwise.
    ///
    /// # Arguments
    ///
    /// * `http` - The HTTP security the configurer is applied to.
    fn access_denied_handler(&self, http: &H) -> Arc<dyn AccessDeniedHandler> {
        http.configurer::<ErrorHandlingConfigurer<H>>()
            .and_then(|configurer| configurer.get_access_denied_handler())
            .cloned()
            .unwrap_or_else(|| {
                Arc::new(AccessDeniedHandlerImpl::default()) as Arc<dyn AccessDeniedHandler>
            })
    }
}

impl<H> Default for PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<PreAuthorizeConfigurer<H>, H>> for PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<PreAuthorizeConfigurer<H>, H> {
        &self.base
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<PreAuthorizeConfigurer<H>, H> {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H> + 'static,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, http: &mut H) {
        // The configurers of the application ran before the chain is built, so
        // the handler of a refusal is the one the application configured.
        let access_denied_handler = self.access_denied_handler(http);
        http.add_filter(PreAuthorizeAuthoritiesFilter::new(access_denied_handler));
    }

    fn configure(&mut self, _http: &mut H) {}
}

impl<H> Deref for PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> std::ops::DerefMut for PreAuthorizeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
