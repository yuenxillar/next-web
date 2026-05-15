use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    core::{granted_authority::GrantedAuthority, userdetails::user_details::UserDetails},
    ldap::userdetails::{
        ldap_granted_authority::LdapGrantedAuthority, ldap_user_details::LdapUserDetails,
    },
};

#[derive(Clone, Debug, Default)]
pub struct LdapUserDetailsImpl {
    dn: String,
    username: String,
    password: String,
    authorities: Vec<Arc<LdapGrantedAuthority>>,
    account_non_expired: bool,
    account_non_locked: bool,
    credentials_non_expired: bool,
    enabled: bool,
}

impl LdapUserDetailsImpl {
    pub fn new(dn: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            dn: dn.into(),
            username: username.into(),
            password: String::new(),
            authorities: Vec::new(),
            account_non_expired: true,
            account_non_locked: true,
            credentials_non_expired: true,
            enabled: true,
        }
    }

    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = password.into();
    }

    pub fn set_authorities(
        &mut self,
        authorities: impl IntoIterator<Item = Arc<LdapGrantedAuthority>>,
    ) {
        self.authorities = authorities.into_iter().collect();
    }
}

#[async_trait]
impl UserDetails for LdapUserDetailsImpl {
    async fn get_authorities(&self) -> Vec<&dyn GrantedAuthority> {
        self.authorities
            .iter()
            .map(|authority| authority.as_ref() as &dyn GrantedAuthority)
            .collect()
    }

    async fn get_password(&self) -> String {
        self.password.clone()
    }

    async fn get_username(&self) -> String {
        self.username.clone()
    }

    async fn is_account_non_expired(&self) -> bool {
        self.account_non_expired
    }

    async fn is_account_non_locked(&self) -> bool {
        self.account_non_locked
    }

    async fn is_credentials_non_expired(&self) -> bool {
        self.credentials_non_expired
    }

    async fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[async_trait]
impl LdapUserDetails for LdapUserDetailsImpl {
    async fn get_dn(&self) -> String {
        self.dn.clone()
    }
}
