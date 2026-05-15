use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    core::userdetails::user_details::UserDetails,
    ldap::{
        dir_context_operations::DirContextOperations,
        userdetails::{
            ldap_granted_authority::LdapGrantedAuthority,
            ldap_user_details_impl::LdapUserDetailsImpl,
        },
    },
};

#[async_trait]
pub trait UserDetailsContextMapper: Send + Sync {
    async fn map_user_from_context(
        &self,
        user_data: &DirContextOperations,
        username: &str,
        authorities: Vec<Arc<LdapGrantedAuthority>>,
    ) -> Arc<dyn UserDetails>;
}

#[derive(Clone, Debug, Default)]
pub struct LdapUserDetailsMapper;

#[async_trait]
impl UserDetailsContextMapper for LdapUserDetailsMapper {
    async fn map_user_from_context(
        &self,
        user_data: &DirContextOperations,
        username: &str,
        authorities: Vec<Arc<LdapGrantedAuthority>>,
    ) -> Arc<dyn UserDetails> {
        let mut details = LdapUserDetailsImpl::new(user_data.dn(), username);
        if let Some(password) = user_data.attribute_first("password") {
            details.set_password(password.to_string());
        }
        details.set_authorities(authorities);
        Arc::new(details)
    }
}
