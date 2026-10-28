pub mod web;

// The management web security is applied when an actuator contributes the
// request matcher the management endpoints are matched with. The framework does
// not contribute one yet, and the auto-configuration would register the filter
// chain the web security already registers, so it is held back until the
// actuator integration exists.
// mod management_web_security_auto_configuration;
mod security_auto_configuration;
mod security_properties;
mod user_details_service_auto_configuration;

pub use security_auto_configuration::SecurityAutoConfiguration;
pub use security_properties::SecurityProperties;
pub use user_details_service_auto_configuration::UserDetailsServiceAutoConfiguration;
