use next_web_macros::configuration_properties;
use uuid::Uuid;

/// Configuration properties for Web Security.
///
/// Binds properties under the `next.security` prefix (for example,
/// `next.security.user.name`) into this struct.
#[configuration_properties(prefix = "next.security")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecurityProperties {
    user: User,
}

impl SecurityProperties {
    /// Creates a new `SecurityProperties` with the given user configuration.
    pub fn new(user: User) -> Self {
        Self { user }
    }

    /// Returns a shared reference to the default user configuration.
    pub fn user(&self) -> &User {
        &self.user
    }

    /// Returns a mutable reference to the default user configuration.
    pub fn user_mut(&mut self) -> &mut User {
        &mut self.user
    }
}

impl Default for SecurityProperties {
    /// Creates default security properties using the default user.
    fn default() -> Self {
        Self {
            user: Default::default(),
        }
    }
}

/// Configuration for the default in-memory user.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct User {
    /// Default user name.
    name: String,
    /// Password for the default user name.
    password: String,
    /// Granted roles for the default user name.
    roles: Vec<String>,
    /// Whether the current password was auto-generated rather than
    /// explicitly configured by the user.
    password_generated: bool,
}

impl User {
    /// Creates a new `User` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the default user name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the default user name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Returns the password for the default user.
    pub fn password(&self) -> &str {
        &self.password
    }

    /// Sets the password for the default user.
    ///
    /// An empty password is ignored, preserving the previous value.
    /// A non-empty password also marks the password as no longer
    /// auto-generated.
    pub fn set_password(&mut self, password: String) {
        if password.is_empty() {
            return;
        }
        self.password_generated = false;
        self.password = password;
    }

    /// Returns the granted roles for the default user.
    pub fn roles(&self) -> &[String] {
        &self.roles
    }

    /// Sets the granted roles for the default user.
    pub fn set_roles(&mut self, roles: impl IntoIterator<Item = String>) {
        self.roles = roles.into_iter().collect();
    }

    /// Returns whether the current password was auto-generated.
    pub fn is_password_generated(&self) -> bool {
        self.password_generated
    }
}

impl Default for User {
    /// Creates a default user with the name `user`, a randomly
    /// generated password, and no roles.
    fn default() -> Self {
        Self {
            name: String::from("user"),
            password: Uuid::new_v4().to_string(),
            roles: Vec::new(),
            password_generated: true,
        }
    }
}
