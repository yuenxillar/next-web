use next_web_core::async_trait;

use crate::{
    core::authentication_error::AuthenticationError,
    ldap::{
        authentication::ldap_authentication_request::LdapAuthenticationRequest,
        dir_context_operations::DirContextOperations,
    },
};

#[async_trait]
pub trait LdapAuthenticator: Send + Sync {
    async fn authenticate(
        &self,
        authentication: &LdapAuthenticationRequest,
    ) -> Result<DirContextOperations, AuthenticationError>;
}
