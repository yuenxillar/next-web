//! Builds the filter chain proxy that performs the web security of the
//! application.

use std::{error::Error, sync::Arc};

use next_web_context::{ApplicationContextExt, SECURITY_FILTERS_SINGLETON_NAME};
use next_web_core::{
    async_trait, config::AutoConfiguration, http::filter::HttpFilter, ApplicationContext, Ordered,
};
use next_web_macros::singleton;

use crate::{
    config::{security_builder::SecurityBuilder, web::builders::WebSecurity},
    web::security_filter_chain::SecurityFilterChain,
};

/// Creates the filter chain proxy of the application.
///
/// The configuration collects the [`SecurityFilterChain`]s the application
/// declares, builds the [`FilterChainProxy`](crate::web::FilterChainProxy) from
/// them, and registers the resulting filter under
/// [`SECURITY_FILTERS_SINGLETON_NAME`], which is where the web layer of the
/// application looks the security filters up.
///
/// A filter chain is declared by a provider that returns an
/// `Arc<dyn SecurityFilterChain>`, or by registering one on the context
/// directly. The configuration applies after the filter chains of the auto
/// configurations it collects, so a chain the security auto-configuration
/// contributes is included as well.
///
/// An application may take over the security filters by registering an
/// `Arc<[Arc<dyn HttpFilter>]>` under [`SECURITY_FILTERS_SINGLETON_NAME`]
/// itself; the configuration then leaves that list alone.
///
/// # Examples
///
/// ```ignore
/// // The security filter chain of the application is declared with a
/// // configuration class, and the auto-configuration builds the proxy from it.
/// #[configuration]
/// impl SecurityConfiguration {
///     #[provider]
///     fn security_filter_chain(mut http: HttpSecurity) -> Arc<dyn SecurityFilterChain> {
///         http.authorize_http_requests(|requests| {
///             requests.any_request().authenticated();
///         });
///
///         Arc::new(http.build())
///     }
/// }
/// ```
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct WebSecurityConfiguration;

impl WebSecurityConfiguration {
    /// Hands the configuration to the context as an auto-configuration.
    #[must_use]
    pub fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for WebSecurityConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        if ctx.contains_singleton_with_name::<Arc<[Arc<dyn HttpFilter>]>>(
            SECURITY_FILTERS_SINGLETON_NAME,
        ) {
            tracing::debug!(
                "the application registers its own security filters, so the filter chain proxy is \
                 not installed"
            );
            return Ok(());
        }

        let security_filter_chains = ctx.resolve_by_type::<Arc<dyn SecurityFilterChain>>();

        let filters: Arc<[Arc<dyn HttpFilter>]> = if security_filter_chains.is_empty() {
            tracing::debug!(
                "no security filter chain is declared, so the application serves its requests \
                 without web security"
            );

            Arc::from(Vec::new())
        } else {
            let mut web_security = WebSecurity::default();
            for security_filter_chain in security_filter_chains {
                web_security.add_security_filter_chain(security_filter_chain);
            }

            let filter_chain_proxy = web_security.build();

            Arc::from(vec![Arc::new(filter_chain_proxy) as Arc<dyn HttpFilter>])
        };

        ctx.insert_singleton_with_name(filters, SECURITY_FILTERS_SINGLETON_NAME);

        Ok(())
    }

    /// The proxy is built after the filter chains the other configurations
    /// contribute, so it is built after the security auto-configuration.
    fn auto_configure_after(&self) -> &'static [&'static str] {
        &["WebSecurityAutoConfiguration"]
    }
}

impl Ordered for WebSecurityConfiguration {
    fn order(&self) -> i32 {
        i32::MAX - 10
    }
}
