// mod security_filter_auto_configuration;
mod web_security_auto_configuration;

use next_web_context::ApplicationContextExt;
// pub use security_filter_auto_configuration::SecurityFilterAutoConfiguration;
pub use web_security_auto_configuration::WebSecurityAutoConfiguration;

pub(super) fn on_default_web_security(ctx: &dyn next_web_core::ApplicationContext) -> bool {
    !ctx.contains_dyn_singleton_with_default_name::<std::sync::Arc<dyn next_web_security::web::security_filter_chain::SecurityFilterChain>>()
    && ctx.contains_singleton_with_default_name::<next_web_security::config::web::builders::HttpSecurity>()
}
