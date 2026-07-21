use std::{collections::BTreeSet, marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    access::hierarchicalroles::{
        null_role_hierarchy::NullRoleHierarchy, role_hierarchy::RoleHierarchy,
    },
    authorization::{authorization_manager::AuthorizationManager, AuthorizationResult},
    core::Authentication,
};

pub struct AllAuthoritiesAuthorizationManager<T> {
    required_authorities: Vec<String>,
    role_hierarchy: Arc<dyn RoleHierarchy>,
    _marker: PhantomData<T>,
}

impl<T> AllAuthoritiesAuthorizationManager<T> {
    pub fn has_all_roles(roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::has_all_prefixed_authorities("ROLE_", roles)
    }

    pub fn has_all_prefixed_authorities(
        prefix: &str,
        authorities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let authorities = authorities
            .into_iter()
            .map(|authority| {
                let authority = authority.into();
                assert!(
                    prefix.is_empty() || !authority.starts_with(prefix),
                    "{} should not start with {} since it is automatically prepended",
                    authority,
                    prefix
                );
                format!("{}{}", prefix, authority)
            })
            .collect::<Vec<_>>();
        Self::has_all_authorities(authorities)
    }

    pub fn has_all_authorities(authorities: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let required_authorities = authorities.into_iter().map(Into::into).collect::<Vec<_>>();
        assert!(
            !required_authorities.is_empty(),
            "authorities cannot be empty"
        );
        Self {
            required_authorities,
            role_hierarchy: Arc::new(NullRoleHierarchy),
            _marker: PhantomData,
        }
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    pub fn missing_authorities(&self, authentication: &dyn Authentication) -> Vec<String> {
        if !authentication.is_authenticated() {
            return self.required_authorities.clone();
        }
        let granted = self
            .role_hierarchy
            .reachable_granted_authorities(authentication.authorities())
            .into_iter()
            .collect::<BTreeSet<_>>();
        self.required_authorities
            .iter()
            .filter(|authority| !granted.contains(*authority))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AllAuthoritiesAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        authorization::all_authorities_authorization_manager::AllAuthoritiesAuthorizationManager,
        core::{authority_utils::AuthorityUtils, simple_authentication::SimpleAuthentication},
    };

    #[test]
    fn requires_all_authorities() {
        let manager =
            AllAuthoritiesAuthorizationManager::<()>::has_all_authorities(["ROLE_A", "ROLE_B"]);
        let authentication = SimpleAuthentication::builder()
            .principal("alice")
            .authorities(AuthorityUtils::create_authority_list(["ROLE_A"]))
            .authenticated(true)
            .build();

        assert_eq!(
            manager.missing_authorities(&authentication),
            vec![String::from("ROLE_B")]
        );
    }
}
