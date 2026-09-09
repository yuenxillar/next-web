use std::{
    any::Any,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::{
    core::{
        authority::{AuthorityUtils, SimpleGrantedAuthority},
        userdetails::UserDetails,
        CredentialsContainer, GrantedAuthority,
    },
    web::authentication::Identity,
};

/// Models core user information retrieved by a UserDetailsService.
/// Developers may use this class directly, subclass it, or write their own UserDetails implementation from scratch.
/// equals and hashcode implementations are based on the username property only, as the intention is that lookups
/// of the same user principal object (in a user registry, for example) will match where the objects
/// represent the same user, not just when all the properties (authorities, password for example) are the same.
///
/// Note that this implementation is not immutable. It implements the CredentialsContainer interface,
/// in order to allow the password to be erased after authentication. This may cause side-effects if you are
/// storing instances in-memory and reusing them. If so, make sure you return a copy from your
/// UserDetailsService each time it is invoked.
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
    /// Calls the more complex constructor with all boolean arguments set to true.
    pub fn new(
        username: impl Into<String>,
        password: Option<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self::with_flags(username, password, true, true, true, true, authorities)
    }

    /// Construct the User with the details required by security.authentication.dao.DaoAuthenticationProvider.
    pub fn with_flags(
        username: impl Into<String>,
        password: Option<String>,
        enabled: bool,
        account_non_expired: bool,
        credentials_non_expired: bool,
        account_non_locked: bool,
        mut authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let username = username.into();
        assert!(
            !username.is_empty() && username != "",
            "Cannot pass null or empty values to constructor"
        );
        Self {
            username,
            password,
            enabled,
            account_non_expired,
            credentials_non_expired,
            account_non_locked,
            authorities: {
                // 1. Sorting
                authorities.sort_by(|a, b| a.authority().cmp(&b.authority()));
                // 2. Deduplicate）
                authorities.dedup_by(|a, b| a.authority() == b.authority());
                authorities
            },
            cleared: AtomicBool::new(false),
        }
    }

    /// Creates a UserBuilder with a specified username
    pub fn with_username(username: impl Into<String>) -> UserBuilder {
        Self::builder().username(username)
    }

    pub fn with_user_details(user_details: &dyn UserDetails) -> UserBuilder {
        let result = Self::with_username(user_details.username())
            .account_expired(!user_details.is_account_non_expired())
            .account_locked(!user_details.is_account_non_locked())
            .authorities(user_details.authorities().to_vec())
            .credentials_expired(!user_details.is_credentials_non_expired())
            .disabled(!user_details.is_enabled());

        if let Some(password) = user_details.password() {
            result.password(Some(password.to_string()))
        } else {
            result
        }
    }

    /// Creates a UserBuilder
    pub fn builder() -> UserBuilder {
        UserBuilder::default()
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

impl Identity for User {
    fn as_any(&self) -> &dyn Any {
        self
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

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User(username={})", self.username)
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

/// A builder for creating [`UserDetails`] instances.
///
/// At minimum, the username, password, and authorities should be provided.
/// The remaining attributes have reasonable defaults.
#[derive(Clone)]
pub struct UserBuilder {
    username: Option<String>,
    password: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    account_expired: bool,
    account_locked: bool,
    credentials_expired: bool,
    disabled: bool,
    password_encoder: Arc<dyn Fn(Option<&str>) -> Option<String> + Send + Sync>,
}

impl UserBuilder {
    /// Populates the username. This attribute is required.
    ///
    /// # Parameters
    /// * `username` - the username. Cannot be empty.
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        let username = username.into();
        assert!(!username.is_empty(), "username cannot be empty");
        self.username = Some(username);
        self
    }

    /// Populates the password. This attribute is required.
    ///
    /// # Parameters
    /// * `password` - the password. Can be `None` if using passwordless authentication.
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn password(mut self, password: Option<String>) -> Self {
        self.password = password;
        self
    }

    /// Encodes the current password (if non-null) and any future passwords supplied to
    /// [`password`](Self::password).
    ///
    /// # Parameters
    /// * `encoder` - the encoder to use
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn password_encoder<F>(mut self, encoder: F) -> Self
    where
        F: Fn(Option<&str>) -> Option<String> + Send + Sync + 'static,
    {
        self.password_encoder = Arc::new(encoder);
        self
    }

    /// Populates the roles. This method is a shortcut for calling
    /// [`authorities`](Self::authorities), but automatically prefixes each entry with
    /// "ROLE_".
    ///
    /// # Example
    /// ```
    /// builder.roles(&["USER", "ADMIN"]);
    /// ```
    /// is equivalent to
    /// ```
    /// builder.authorities(&["ROLE_USER", "ROLE_ADMIN"]);
    /// ```
    ///
    /// # Parameters
    /// * `roles` - the roles for this user (i.e. USER, ADMIN, etc). Cannot be empty,
    ///   contain empty values or start with "ROLE_".
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn roles(mut self, roles: &[&str]) -> Self {
        let mut authorities = Vec::with_capacity(roles.len());
        for role in roles {
            assert!(
                !role.starts_with("ROLE_"),
                "{} cannot start with ROLE_ (it is automatically added)",
                role
            );
            let role_name = format!("ROLE_{}", role);
            authorities.push(Arc::new(SimpleGrantedAuthority::new(role_name)) as Arc<dyn GrantedAuthority>);
        }
        self.authorities = authorities;
        self
    }

    /// Populates the authorities. This attribute is required.
    ///
    /// # Parameters
    /// * `authorities` - the authorities for this user. Cannot be empty.
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn authorities<I>(mut self, authorities: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn GrantedAuthority>>,
    {
        let authorities = authorities.into_iter().collect::<Vec<_>>();
        assert!(!authorities.is_empty(), "authorities cannot be empty");
        self.authorities = authorities;
        self
    }

    /// Populates the authorities from authority strings.
    ///
    /// # Parameters
    /// * `authorities` - the authorities for this user (i.e. ROLE_USER, ROLE_ADMIN, etc).
    ///   Cannot be empty.
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn authorities_with_strings(self, authorities: &[&str]) -> Self {
        assert!(!authorities.is_empty(), "authorities cannot be empty");
        self.authorities(AuthorityUtils::create_authority_list(authorities))
    }

    /// Defines if the account is expired or not. Default is `false`.
    ///
    /// # Parameters
    /// * `account_expired` - `true` if the account is expired, `false` otherwise
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn account_expired(mut self, account_expired: bool) -> Self {
        self.account_expired = account_expired;
        self
    }

    /// Defines if the account is locked or not. Default is `false`.
    ///
    /// # Parameters
    /// * `account_locked` - `true` if the account is locked, `false` otherwise
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn account_locked(mut self, account_locked: bool) -> Self {
        self.account_locked = account_locked;
        self
    }

    /// Defines if the credentials are expired or not. Default is `false`.
    ///
    /// # Parameters
    /// * `credentials_expired` - `true` if the credentials are expired, `false` otherwise
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn credentials_expired(mut self, credentials_expired: bool) -> Self {
        self.credentials_expired = credentials_expired;
        self
    }

    /// Defines if the account is disabled or not. Default is `false`.
    ///
    /// # Parameters
    /// * `disabled` - `true` if the account is disabled, `false` otherwise
    ///
    /// # Returns
    /// The [`UserBuilder`] for method chaining.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Builds the [`UserDetails`] instance.
    ///
    /// # Panics
    /// Panics if `username` is `None`.
    ///
    /// # Returns
    /// The built [`UserDetails`].
    pub fn build(self) -> User {
        let username = self.username.expect("username cannot be none");
        let encoded_password = self
            .password
            .and_then(|pwd| (self.password_encoder)(Some(&pwd)));

        User::with_flags(
            username,
            encoded_password,
            !self.disabled,
            !self.account_expired,
            !self.credentials_expired,
            !self.account_locked,
            self.authorities,
        )
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self {
            username: None,
            password: None,
            authorities: Vec::new(),
            account_expired: false,
            account_locked: false,
            credentials_expired: false,
            disabled: false,
            password_encoder: Arc::new(|password| password.map(ToString::to_string)),
        }
    }
}
