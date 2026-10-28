use std::sync::Arc;

use std::collections::HashSet;

use next_web_core::{async_trait, error::BoxError};

use crate::{
    access::hierarchicalroles::RoleHierarchy,
    authorization::{AuthorityAuthorizationDecision, AuthorizationManager, AuthorizationResult},
    core::{authority::AuthorityUtils, Authentication},
};

/// An AuthorizationManager that determines if the current user is authorized by evaluating if the
/// Authentication contains any of the specified authorities.
#[derive(Clone)]
pub struct AuthoritiesAuthorizationManager {
    role_hierarchy: Option<Arc<dyn RoleHierarchy>>,
}

impl AuthoritiesAuthorizationManager {
    /// Sets the RoleHierarchy to be used. Default is None
    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = Some(role_hierarchy);
    }

    fn is_granted(&self, authentication: &dyn Authentication, authority: &HashSet<String>) -> bool {
        self.is_authorized(authentication, authority)
    }

    fn is_authorized(
        &self,
        authentication: &dyn Authentication,
        authorities: &HashSet<String>,
    ) -> bool {
        if authorities.is_empty() {
            return false;
        }

        self.role_hierarchy.as_ref().map_or(false, |rh| {
            rh.reachable_granted_authorities(authentication.authorities())
                .iter()
                .any(|ga| {
                    ga.authority()
                        .map_or(false, |auth| authorities.contains(auth))
                })
        })
    }
}
#[async_trait]
impl AuthorizationManager<HashSet<String>> for AuthoritiesAuthorizationManager {
    /// Determines if the current user is authorized by evaluating if the
    /// Authentication contains any of specified authorities.
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        authorities: &HashSet<String>,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let granted = self.is_granted(authentication, authorities);

        Ok(Some(Arc::new(AuthorityAuthorizationDecision::new(
            granted,
            AuthorityUtils::create_authority_list(authorities.iter()),
        ))))
    }
}

impl Default for AuthoritiesAuthorizationManager {
    fn default() -> Self {
        Self {
            role_hierarchy: None,
        }
    }
}
