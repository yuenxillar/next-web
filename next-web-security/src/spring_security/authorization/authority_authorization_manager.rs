use std::{collections::HashSet, fmt::Debug, marker::PhantomData, sync::Arc};

use next_web_core::{async_trait, error::BoxError};

use crate::{
    access::hierarchicalroles::RoleHierarchy,
    authorization::{AuthoritiesAuthorizationManager, AuthorizationManager, AuthorizationResult},
    core::Authentication,
};

pub(crate) const ROLE_PREFIX: &str = "ROLE_";

/// An AuthorizationManager that determines if the current user is authorized by evaluating if the
/// Authentication contains a specified authority.
pub struct AuthorityAuthorizationManager<T> {
    delegate: AuthoritiesAuthorizationManager,
    authorities: HashSet<String>,

    _marker: PhantomData<T>,
}

impl<T> AuthorityAuthorizationManager<T> {
    /// Creates a new AuthorityAuthorizationManager with the provided authorities.
    pub fn new<I, V>(authorities: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        Self {
            authorities: authorities
                .into_iter()
                .map(Into::into)
                .collect::<HashSet<_>>(),
            delegate: AuthoritiesAuthorizationManager::default(),
            _marker: PhantomData,
        }
    }

    /// Sets the RoleHierarchy to be used. Default is NullRoleHierarchy. Cannot be null.
    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.delegate.set_role_hierarchy(role_hierarchy);
    }
}

impl<T> AuthorityAuthorizationManager<T> {
    /// Creates an instance of AuthorityAuthorizationManager with the provided authority.
    pub fn has_role(role: &str) -> AuthorityAuthorizationManager<T> {
        assert!(
            !role.starts_with(ROLE_PREFIX),
            "{} should not start with {} since {} is automatically prepended when using hasRole. Consider using hasAuthority instead.",
            role, ROLE_PREFIX, ROLE_PREFIX
        );
        Self::has_authority(&format!("{}{}", ROLE_PREFIX, role))
    }

    /// Creates an instance of AuthorityAuthorizationManager with the provided authority.
    pub fn has_authority(authority: &str) -> AuthorityAuthorizationManager<T> {
        assert!(!authority.is_empty(), "authority cannot be null");
        AuthorityAuthorizationManager::new([authority])
    }

    /// Creates an instance of AuthorityAuthorizationManager with the provided authority.
    pub fn has_any_role(role_prefix: &str, roles: Vec<String>) -> AuthorityAuthorizationManager<T> {
        assert!(roles.len() > 0, "roles cannot be empty");
        Self::has_any_authority(Self::to_named_roles_array(role_prefix, roles))
    }

    /// Creates an instance of AuthorityAuthorizationManager with the provided authorities.
    pub fn has_any_authority(authorities: Vec<String>) -> AuthorityAuthorizationManager<T> {
        assert!(authorities.len() > 0, "authorities cannot be empty");
        AuthorityAuthorizationManager::new(authorities)
    }

    fn to_named_roles_array(role_prefix: &str, roles: Vec<String>) -> Vec<String> {
        let mut result = Vec::with_capacity(roles.len());
        for role in roles {
            assert!(role_prefix.is_empty() || !role.starts_with(role_prefix),
            "{} should not start with {} since {} is automatically prepended when using hasAnyRole. Consider using hasAnyAuthority instead.",
            role, role_prefix, role_prefix
        );
            result.push(format!("{}{}", role_prefix, role));
        }

        result
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AuthorityAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    /// Determines if access is granted for a specific authentication and object.
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        _var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        self.delegate
            .authorize(authentication, &self.authorities)
            .await
    }
}

impl<T> Debug for AuthorityAuthorizationManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorityAuthorizationManager")
            .field("authorities", &self.authorities)
            .finish()
    }
}
