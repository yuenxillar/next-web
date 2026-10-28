use std::{marker::PhantomData, sync::Arc};

use next_web_core::{async_trait, error::BoxError};

use crate::{
    access::hierarchicalroles::{NullRoleHierarchy, RoleHierarchy},
    authorization::{
        authority_authorization_decision::AuthorityAuthorizationDecision,
        authorization_manager::AuthorizationManager, AuthorizationResult,
    },
    core::{authority::AuthorityUtils, Authentication},
};

/// An AuthorizationManager that determines if the current user is authorized by evaluating if the
/// Authentication contains all the specified authorities.
pub struct AllAuthoritiesAuthorizationManager<T> {
    role_hierarchy: Arc<dyn RoleHierarchy>,
    required_authorities: Vec<String>,
    _marker: PhantomData<T>,
}

impl<T> AllAuthoritiesAuthorizationManager<T> {
    const ROLE_PREFIX: &str = "ROLE_";

    /// Creates a new instance.
    fn new<I, V>(required_authorities: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        let required_authorities = required_authorities
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        assert!(
            !required_authorities.is_empty(),
            "requiredAuthorities cannot be empty"
        );
        Self {
            role_hierarchy: Arc::new(NullRoleHierarchy::default()),
            required_authorities,
            _marker: PhantomData,
        }
    }

    /// Sets the RoleHierarchy to be used. Default is NullRoleHierarchy.
    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    /// Creates an instance of AllAuthoritiesAuthorizationManager with the provided roles.
    /// Each role should not start with "ROLE_" since it is automatically prepended.
    pub fn has_all_roles<I, V>(roles: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        Self::has_all_prefixed_authorities(Self::ROLE_PREFIX, roles)
    }

    /// Creates an instance of AllAuthoritiesAuthorizationManager with the provided authorities.
    /// Each authority should not start with prefix since it is automatically prepended.
    pub fn has_all_prefixed_authorities<I, V>(prefix: &str, authorities: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        let authorities = authorities.into_iter().map(Into::into).collect::<Vec<_>>();
        assert!(!authorities.is_empty(), "roles cannot contain null values");
        Self::has_all_authorities(Self::to_named_roles_array(prefix, authorities))
    }

    /// Creates an instance of AllAuthoritiesAuthorizationManager with the provided authorities.
    pub fn has_all_authorities<I, V>(authorities: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        Self::new(authorities)
    }

    fn get_granted_authorities(&self, authentication: &dyn Authentication) -> Vec<String> {
        if !authentication.is_authenticated() {
            return Vec::new();
        }
        self.role_hierarchy
            .reachable_granted_authorities(authentication.authorities())
            .into_iter()
            .filter_map(|granted_authority| granted_authority.authority().map(ToString::to_string))
            .collect()
    }

    fn to_named_roles_array(role_prefix: &str, roles: Vec<String>) -> Vec<String> {
        let mut result = Vec::with_capacity(roles.len());
        for role in roles {
            assert!(
                role_prefix.is_empty() || !role.starts_with(role_prefix),
                "{} should not start with {} since {} is automatically prepended when using hasAnyRole. Consider using hasAnyAuthority instead.",
                role,
                role_prefix,
                role_prefix
            );
            result.push(format!("{}{}", role_prefix, role));
        }

        result
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AllAuthoritiesAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    /// Determines if the current user is authorized by evaluating if the Authentication contains
    /// all of the specified authorities.
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        _var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let authenticated_authorities = self.get_granted_authorities(authentication);
        let mut missing_authorities = Vec::with_capacity(self.required_authorities.len());
        missing_authorities.retain(|auth| !authenticated_authorities.contains(auth));

        Ok(Some(Arc::new(AuthorityAuthorizationDecision::new(
            missing_authorities.is_empty(),
            AuthorityUtils::create_authority_list(missing_authorities),
        ))))
    }
}
