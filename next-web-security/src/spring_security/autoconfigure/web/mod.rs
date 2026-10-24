// mod security_filter_auto_configuration;
mod web_security_auto_configuration;

use next_web_context::ApplicationContextExt;
// pub use security_filter_auto_configuration::SecurityFilterAutoConfiguration;
pub use web_security_auto_configuration::WebSecurityAutoConfiguration;

pub(super) fn on_default_web_security(ctx: &dyn next_web_core::ApplicationContext) -> bool {
    // A filter chain the application declares, either as an instance or as a
    // provider, replaces the default one.
    let security_filter_chain_type = std::any::TypeId::of::<
        std::sync::Arc<dyn crate::web::security_filter_chain::SecurityFilterChain>,
    >();

    ctx.keys_of_type(security_filter_chain_type).is_empty()
        && ctx.contains_singleton_with_default_name::<crate::config::web::builders::HttpSecurity>()
}
