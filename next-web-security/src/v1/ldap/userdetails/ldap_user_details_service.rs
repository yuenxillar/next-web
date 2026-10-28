use std::sync::Arc;

use crate::{
    core::{
        userdetails::{UserDetails, UserDetailsService},
        AuthenticationError,
    },
    ldap::{
        search::ldap_user_search::LdapUserSearch,
        userdetails::{
            ldap_authorities_populator::{LdapAuthoritiesPopulator, NullLdapAuthoritiesPopulator},
            user_details_context_mapper::{LdapUserDetailsMapper, UserDetailsContextMapper},
        },
    },
};

pub struct LdapUserDetailsService {
    user_search: Arc<dyn LdapUserSearch>,
    authorities_populator: Arc<dyn LdapAuthoritiesPopulator>,
    user_details_mapper: Arc<dyn UserDetailsContextMapper>,
}

impl LdapUserDetailsService {
    pub fn new(user_search: Arc<dyn LdapUserSearch>) -> Self {
        Self::with_authorities_populator(user_search, Arc::new(NullLdapAuthoritiesPopulator))
    }

    pub fn with_authorities_populator(
        user_search: Arc<dyn LdapUserSearch>,
        authorities_populator: Arc<dyn LdapAuthoritiesPopulator>,
    ) -> Self {
        Self {
            user_search,
            authorities_populator,
            user_details_mapper: Arc::new(LdapUserDetailsMapper),
        }
    }

    pub fn set_user_details_mapper(
        &mut self,
        user_details_mapper: Arc<dyn UserDetailsContextMapper>,
    ) {
        self.user_details_mapper = user_details_mapper;
    }
}

#[next_web_core::async_trait]
impl UserDetailsService for LdapUserDetailsService {
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        let user_data = self.user_search.search_for_user(username).await?;
        let authorities = self
            .authorities_populator
            .get_granted_authorities(&user_data, username)
            .await;
        Ok(self
            .user_details_mapper
            .map_user_from_context(&user_data, username, authorities)
            .await)
    }
}
