use std::{marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    access::hierarchicalroles::{NullRoleHierarchy, RoleHierarchy},
    authorization::{
        authority_authorization_decision::AuthorityAuthorizationDecision,
        authorization_manager::AuthorizationManager, AuthorizationResult,
    },
    core::{authority_utils::AuthorityUtils, Authentication},
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

    /// Creates a new instance with the provided authorities that are required.
    fn new(required_authorities: impl IntoIterator<Item = impl Into<String>>) -> Self {
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
    pub fn has_all_roles(roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::has_all_prefixed_authorities("ROLE_", roles)
    }

    /// Creates an instance of AllAuthoritiesAuthorizationManager with the provided authorities.
    /// Each authority should not start with prefix since it is automatically prepended.
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
                    "{} should not start with {} since {} is automatically prepended when using hasAnyRole. Consider using hasAnyAuthority instead.",
                    authority,
                    prefix,
                    prefix
                );
                format!("{}{}", prefix, authority)
            })
            .collect::<Vec<_>>();
        Self::has_all_authorities(authorities)
    }

    /// Creates an instance of AllAuthoritiesAuthorizationManager with the provided authorities.
    pub fn has_all_authorities(authorities: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::new(authorities)
    }

    fn get_granted_authorities(&self, authentication: &dyn Authentication) -> Vec<String> {
        if !authentication.is_authenticated() {
            return Vec::new();
        }
        self.role_hierarchy
            .reachable_granted_authorities(authentication.authorities())
            .into_iter()
            .filter_map(|authority| authority.authority().map(ToString::to_string))
            .collect()
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
    ) -> Option<Box<dyn AuthorizationResult>> {
        let granted_authorities = self.get_granted_authorities(authentication);
        let mut missing_authorities = self.required_authorities.clone();
        missing_authorities.retain(|authority| !granted_authorities.contains(authority));
        Some(Box::new(AuthorityAuthorizationDecision::new(
            missing_authorities.is_empty(),
            AuthorityUtils::create_authority_list(missing_authorities),
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        access::hierarchicalroles::role_hierarchy_impl::RoleHierarchyImpl,
        authorization::{
            all_authorities_authorization_manager::AllAuthoritiesAuthorizationManager,
            authority_authorization_decision::AuthorityAuthorizationDecision,
            authorization_manager::AuthorizationManager,
        },
        core::{authority_utils::AuthorityUtils, Authentication, SimpleAuthentication},
    };

    fn authentication(authorities: &[&str], authenticated: bool) -> Arc<dyn Authentication> {
        let mut builder = SimpleAuthentication::default().to_builder();
        builder.principal(Some(Arc::new(String::from("alice"))));
        builder.authorities(Box::new(|auths| {
            auths.extend(AuthorityUtils::create_authority_list(
                authorities.iter().copied(),
            ));
        }));
        builder.authenticated(authenticated);
        builder.build()
    }

    async fn decision(
        manager: &AllAuthoritiesAuthorizationManager<()>,
        authentication: &dyn Authentication,
    ) -> (bool, Vec<String>) {
        let mut var = ();
        let result = manager.authorize(authentication, &mut var).await.unwrap();
        let decision = (result.as_ref() as &dyn std::any::Any)
            .downcast_ref::<AuthorityAuthorizationDecision>()
            .unwrap();
        (
            decision.is_granted(),
            decision
                .authorities()
                .iter()
                .filter_map(|authority| authority.authority().map(ToString::to_string))
                .collect(),
        )
    }

    #[tokio::test]
    async fn grants_when_all_required_authorities_are_present() {
        let manager =
            AllAuthoritiesAuthorizationManager::<()>::has_all_authorities(["ROLE_A", "ROLE_B"]);
        let authentication = authentication(&["ROLE_A", "ROLE_B"], true);

        assert_eq!(
            decision(&manager, authentication.as_ref()).await,
            (true, Vec::new())
        );
    }

    #[tokio::test]
    async fn denies_and_reports_missing_authorities() {
        let manager =
            AllAuthoritiesAuthorizationManager::<()>::has_all_authorities(["ROLE_A", "ROLE_B"]);
        let authentication = authentication(&["ROLE_A"], true);

        assert_eq!(
            decision(&manager, authentication.as_ref()).await,
            (false, vec![String::from("ROLE_B")])
        );
    }

    #[tokio::test]
    async fn denies_unauthenticated_user_with_all_authorities_missing() {
        let manager =
            AllAuthoritiesAuthorizationManager::<()>::has_all_authorities(["ROLE_A", "ROLE_B"]);
        let authentication = authentication(&["ROLE_A"], false);

        assert_eq!(
            decision(&manager, authentication.as_ref()).await,
            (false, vec![String::from("ROLE_A"), String::from("ROLE_B")])
        );
    }

    #[tokio::test]
    async fn has_all_roles_prepends_the_role_prefix() {
        let manager = AllAuthoritiesAuthorizationManager::<()>::has_all_roles(["A", "B"]);
        let authentication = authentication(&["ROLE_A", "ROLE_B"], true);

        assert_eq!(
            decision(&manager, authentication.as_ref()).await,
            (true, Vec::new())
        );
    }

    #[tokio::test]
    async fn role_hierarchy_supplies_inherited_authorities() {
        let mut manager =
            AllAuthoritiesAuthorizationManager::<()>::has_all_authorities(["ROLE_A", "ROLE_B"]);
        manager.set_role_hierarchy(Arc::new(
            RoleHierarchyImpl::from_hierarchy("ROLE_A > ROLE_B").unwrap(),
        ));
        let authentication = authentication(&["ROLE_A"], true);

        assert_eq!(
            decision(&manager, authentication.as_ref()).await,
            (true, Vec::new())
        );
    }

    #[test]
    fn empty_authorities_are_rejected() {
        let result = std::panic::catch_unwind(|| {
            AllAuthoritiesAuthorizationManager::<()>::has_all_authorities(Vec::<String>::new())
        });
        assert!(result.is_err());
    }

    #[test]
    fn role_prefixed_authority_is_rejected() {
        let result = std::panic::catch_unwind(|| {
            AllAuthoritiesAuthorizationManager::<()>::has_all_roles(["ROLE_A"])
        });
        assert!(result.is_err());
    }
}
