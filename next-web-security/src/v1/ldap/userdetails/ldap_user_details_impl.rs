use std::{
    any::Any,
    fmt::{self},
    sync::Arc,
};

use next_web_core::async_trait;

use crate::{
    core::{userdetails::UserDetails, GrantedAuthority},
    ldap::userdetails::{
        ldap_granted_authority::LdapGrantedAuthority, ldap_user_details::LdapUserDetails,
    },
    web::authentication::Identity,
};

#[derive(Clone, Default)]
pub struct LdapUserDetailsImpl {
    dn: String,
    username: String,
    password: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
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
        // The authorities are stored as the trait objects of the user details
        // contract, so the getter can hand out the slice it holds.
        self.authorities = authorities
            .into_iter()
            .map(|authority| authority as Arc<dyn GrantedAuthority>)
            .collect();
    }
}

impl UserDetails for LdapUserDetailsImpl {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }

    fn password(&self) -> Option<&str> {
        Some(self.password.as_str())
    }

    fn username(&self) -> &str {
        self.username.as_str()
    }

    fn is_account_non_expired(&self) -> bool {
        self.account_non_expired
    }

    fn is_account_non_locked(&self) -> bool {
        self.account_non_locked
    }

    fn is_credentials_non_expired(&self) -> bool {
        self.credentials_non_expired
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[async_trait]
impl LdapUserDetails for LdapUserDetailsImpl {
    async fn get_dn(&self) -> String {
        self.dn.clone()
    }
}

impl Identity for LdapUserDetailsImpl {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Display for LdapUserDetailsImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LdapUserDetailsImpl(username={})", self.username)
    }
}

impl fmt::Debug for LdapUserDetailsImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LdapUserDetailsImpl")
            .field("dn", &self.dn)
            .field("username", &self.username)
            .field("authorities", &self.authorities.len())
            .field("account_non_expired", &self.account_non_expired)
            .field("account_non_locked", &self.account_non_locked)
            .field("credentials_non_expired", &self.credentials_non_expired)
            .field("enabled", &self.enabled)
            .finish()
    }
}
