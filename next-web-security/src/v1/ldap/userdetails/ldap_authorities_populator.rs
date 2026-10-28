use std::sync::Arc;

use next_web_core::async_trait;

use crate::ldap::{
    dir_context_operations::DirContextOperations,
    userdetails::ldap_granted_authority::LdapGrantedAuthority,
};

#[async_trait]
pub trait LdapAuthoritiesPopulator: Send + Sync {
    async fn get_granted_authorities(
        &self,
        user_data: &DirContextOperations,
        username: &str,
    ) -> Vec<Arc<LdapGrantedAuthority>>;
}

#[derive(Clone, Debug, Default)]
pub struct NullLdapAuthoritiesPopulator;

#[async_trait]
impl LdapAuthoritiesPopulator for NullLdapAuthoritiesPopulator {
    async fn get_granted_authorities(
        &self,
        _user_data: &DirContextOperations,
        _username: &str,
    ) -> Vec<Arc<LdapGrantedAuthority>> {
        Vec::new()
    }
}
