mod http_security_configuration;
mod web_security_configuration;

#[cfg(feature = "oauth2-authorization-server")]
mod oauth2_authorization_server_configuration;
#[cfg(feature = "oauth2-client")]
mod oauth2_client_configuration;

pub use http_security_configuration::HttpSecurityConfiguration;
pub use web_security_configuration::WebSecurityConfiguration;

#[cfg(feature = "oauth2-authorization-server")]
pub use oauth2_authorization_server_configuration::OAuth2AuthorizationServerConfiguration;
#[cfg(feature = "oauth2-client")]
pub use oauth2_client_configuration::OAuth2ClientConfiguration;
