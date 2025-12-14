use std::{collections::HashSet, hash::Hash, marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    access::hierarchicalroles::role_hierarchy::RoleHierarchy,
    authorization::{
        authorities_authorization_manager::AuthoritiesAuthorizationManager,
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
    },
    core::authentication::Authentication,
};

pub struct AuthorityAuthorizationManager<T> {
    context: PhantomData<T>,

    delegate: AuthoritiesAuthorizationManager,
    authorities: HashSet<String>,
}

impl<T> AuthorityAuthorizationManager<T> {
    pub fn new(authorities: HashSet<String>) -> Self {
        Self {
            context: PhantomData,
            authorities,
            delegate: todo!(),
        }
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.delegate.set_role_hierarchy(role_hierarchy);
    }
}

impl<T> AuthorityAuthorizationManager<T> {
    pub fn has_any_role(
        role_prefix: &str,
        roles: impl IntoIterator<Item = String>,
    ) -> AuthorityAuthorizationManager<T> {
        let roles = roles.into_iter().collect::<Vec<_>>();
        assert!(roles.len() > 0, "roles cannot contain null values");
        Self::has_any_authority(Self::to_named_roles_array(role_prefix, roles))
    }

    pub fn has_authority(authority: &str) -> AuthorityAuthorizationManager<T> {
        assert!(!authority.is_empty(), "authority cannot be null");
        AuthorityAuthorizationManager::new(HashSet::from_iter([authority.to_string()]))
    }

    pub fn has_any_authority(authorities: Vec<String>) -> AuthorityAuthorizationManager<T> {
        assert!(
            authorities.len() > 0,
            "authorities cannot contain null values"
        );
        AuthorityAuthorizationManager::new(HashSet::from_iter(authorities))
    }

    fn to_named_roles_array(role_prefix: &str, roles: Vec<String>) -> Vec<String> {
        let mut result = Vec::new();
        for role in roles {
            assert!(role.is_empty() || !role.starts_with(role_prefix),
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
    async fn check(
        &self,
        authentication: Box<dyn Authentication>,
        object: T,
    ) -> Option<AuthorizationDecision> {
        None
    }
}
