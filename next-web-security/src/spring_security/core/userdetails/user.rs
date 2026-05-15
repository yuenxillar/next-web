use std::{fmt, sync::Arc};

use next_web_core::async_trait;

use crate::core::{
    credentials_container::CredentialsContainer, granted_authority::GrantedAuthority,
    userdetails::user_details::UserDetails,
};

#[derive(Clone)]
pub struct User {
    password: Option<String>,
    username: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    account_non_expired: bool,
    account_non_locked: bool,
    credentials_non_expired: bool,
    enabled: bool,
}

impl User {
    pub fn new(
        username: impl Into<String>,
        password: Option<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self::with_flags(username, password, true, true, true, true, authorities)
    }

    pub fn with_flags(
        username: impl Into<String>,
        password: Option<String>,
        enabled: bool,
        account_non_expired: bool,
        credentials_non_expired: bool,
        account_non_locked: bool,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let username = username.into();
        assert!(
            !username.trim().is_empty(),
            "Cannot pass null or empty values to constructor"
        );
        Self {
            password,
            username,
            authorities,
            account_non_expired,
            account_non_locked,
            credentials_non_expired,
            enabled,
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

#[async_trait]
impl UserDetails for User {
    async fn get_authorities(&self) -> Vec<&dyn GrantedAuthority> {
        self.authorities
            .iter()
            .map(|authority| authority.as_ref() as &dyn GrantedAuthority)
            .collect()
    }

    async fn get_password(&self) -> String {
        self.password.clone().unwrap_or_default()
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

impl CredentialsContainer for User {
    fn erase_credentials(&mut self) {
        self.password = None;
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.username == other.username
    }
}

impl Eq for User {}

impl std::hash::Hash for User {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.username.hash(state);
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("username", &self.username)
            .field("password", &"[PROTECTED]")
            .field("enabled", &self.enabled)
            .field("account_non_expired", &self.account_non_expired)
            .field("credentials_non_expired", &self.credentials_non_expired)
            .field("account_non_locked", &self.account_non_locked)
            .field("authorities_len", &self.authorities.len())
            .finish()
    }
}
