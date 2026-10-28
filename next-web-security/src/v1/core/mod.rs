pub mod authority;
pub mod context;
pub mod session;
pub mod token;
pub mod userdetails;

mod authenticated_principal;
mod authentication;
mod authentication_error;
mod credentials_container;
mod granted_authority;
mod next_security_message_source;
mod principal;
mod simple_authentication;

pub use authenticated_principal::AuthenticatedPrincipal;
pub use authentication::{Authentication, AuthenticationBuilder};
pub use authentication_error::{AuthenticationError, AuthenticationErrorKind};
pub use credentials_container::CredentialsContainer;
pub use granted_authority::GrantedAuthority;
pub use next_security_message_source::NextSecurityMessageSource;
pub use principal::Principal;
pub use simple_authentication::{SimpleAuthentication, SimpleAuthenticationBuilder};

// pub mod memory_auth_service;
// pub mod user_permission_resource;
