pub mod authority;
pub mod authority_mapping;
pub mod authority_utils;
pub mod context;

pub mod granted_authorities_container;
pub mod session;

pub mod simple_granted_authority;
pub mod token;
pub mod user_cache;
pub mod userdetails;
pub mod username_not_found_error;

mod authenticated_principal;
mod authentication;
mod authentication_error;
mod credentials_container;
mod erasable_value;
mod granted_authority;
mod next_security_message_source;
mod principal;
mod simple_authentication;
mod username_password_authentication_token;

pub use authenticated_principal::AuthenticatedPrincipal;
pub use authentication::{Authentication, AuthenticationBuilder};
pub use authentication_error::{AuthenticationError, AuthenticationErrorKind};
pub use credentials_container::CredentialsContainer;
pub use erasable_value::ErasableValue;
pub use granted_authority::GrantedAuthority;
pub use next_security_message_source::NextSecurityMessageSource;
pub use principal::Principal;
pub use simple_authentication::{SimpleAuthentication, SimpleAuthenticationBuilder};
pub use username_password_authentication_token::UsernamePasswordAuthenticationToken;

// pub mod memory_auth_service;
// pub mod user_permission_resource;
