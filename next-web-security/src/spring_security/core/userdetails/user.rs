use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::core::{
    credentials_container::CredentialsContainer, userdetails::UserDetails, GrantedAuthority,
};

pub struct User {
    password: Option<String>,
    username: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    account_non_expired: bool,
    account_non_locked: bool,
    credentials_non_expired: bool,
    enabled: bool,

    cleared: AtomicBool,
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
            cleared: AtomicBool::new(false),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

impl UserDetails for User {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }

    fn password(&self) -> Option<&str> {
        if self.cleared.load(Ordering::Acquire) {
            return None;
        }
        self.password.as_deref()
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

impl CredentialsContainer for User {
    fn erase_credentials(&self) {
        self.cleared.store(true, Ordering::Release);
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
            .field("cleared", &self.cleared.load(Ordering::Acquire))
            .finish()
    }
}

impl Clone for User {
    fn clone(&self) -> Self {
        Self {
            password: self.password.clone(),
            username: self.username.clone(),
            authorities: self.authorities.clone(),
            account_non_expired: self.account_non_expired,
            account_non_locked: self.account_non_locked,
            credentials_non_expired: self.credentials_non_expired,
            enabled: self.enabled,
            cleared: AtomicBool::new(self.cleared.load(Ordering::Acquire)),
        }
    }
}
