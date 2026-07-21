pub mod authentication_converter;
pub mod authentication_failure_handler;
pub mod base_authentication_filter;
pub mod base_authentication_processing_filter;
pub mod basic_authentication_entry_point;
pub mod forward_authentication_failure_handler;
pub mod forward_authentication_success_handler;
pub mod login_url_authentication_entry_point;
pub mod logout;
pub mod ott;
pub mod preauth;
pub mod rememberme;
pub mod session;
pub mod simple_url_authentication_failure_handler;
pub mod simple_url_authentication_success_handler;
pub mod switchuser;
pub mod ui;
pub mod www;

mod anonymous_authentication_filter;
mod authentication_filter;
mod authentication_success_handler;
mod base_authentication_target_url_request_handler;
mod delegating_authentication_entry_point;
mod remember_me_authentication_filter;
mod remember_me_services;
mod saved_request_aware_authentication_success_handler;
mod username_password_authentication_filter;
mod web_authentication_details;
mod web_authentication_details_source;

pub use anonymous_authentication_filter::AnonymousAuthenticationFilter;
pub use authentication_filter::AuthenticationFilter;
pub use authentication_success_handler::AuthenticationSuccessHandler;
pub use base_authentication_target_url_request_handler::BaseAuthenticationTargetUrlRequestHandler;
pub use delegating_authentication_entry_point::{
    DelegatingAuthenticationEntryPoint, DelegatingAuthenticationEntryPointBuilder,
};
pub use remember_me_authentication_filter::RememberMeAuthenticationFilter;
pub use remember_me_services::RememberMeServices;
pub use saved_request_aware_authentication_success_handler::SavedRequestAwareAuthenticationSuccessHandler;
pub use username_password_authentication_filter::UsernamePasswordAuthenticationFilter;
pub use web_authentication_details::WebAuthenticationDetails;
pub use web_authentication_details_source::WebAuthenticationDetailsSource;

pub type AuthPrincipal = std::sync::Arc<dyn std::any::Any + Send + Sync>;
