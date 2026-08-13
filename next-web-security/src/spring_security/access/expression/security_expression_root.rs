use std::{collections::HashSet, sync::Arc};

use next_web_core::anys::any_value::AnyValue;

use crate::{
    access::{
        expression::{
            deny_all_permission_evaluator::DenyAllPermissionEvaluator,
            security_expression_operations::SecurityExpressionOperations,
        },
        hierarchicalroles::{NullRoleHierarchy, RoleHierarchy},
        permission_evaluator::PermissionEvaluator,
    },
    authorization::{AuthenticationTrustResolver, DefaultAuthenticationTrustResolver},
    core::Authentication,
};

pub struct SecurityExpressionRoot {
    authentication: Arc<dyn Authentication>,
    default_role_prefix: String,
    permission_evaluator: Arc<dyn PermissionEvaluator>,
    role_hierarchy: Arc<dyn RoleHierarchy>,
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
}

impl SecurityExpressionRoot {
    pub const READ: &'static str = "read";
    pub const WRITE: &'static str = "write";
    pub const CREATE: &'static str = "create";
    pub const DELETE: &'static str = "delete";
    pub const ADMIN: &'static str = "administration";

    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            authentication,
            default_role_prefix: String::from("ROLE_"),
            permission_evaluator: Arc::new(DenyAllPermissionEvaluator),
            role_hierarchy: Arc::new(NullRoleHierarchy),
            trust_resolver: Arc::new(DefaultAuthenticationTrustResolver::default()),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }

    pub fn principal(&self) -> Option<String> {
        self.authentication.get_principal()
    }

    pub fn set_default_role_prefix(&mut self, default_role_prefix: impl Into<String>) {
        self.default_role_prefix = default_role_prefix.into();
    }

    pub fn set_permission_evaluator(&mut self, permission_evaluator: Arc<dyn PermissionEvaluator>) {
        self.permission_evaluator = permission_evaluator;
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }

    fn authorities(&self) -> HashSet<String> {
        self.role_hierarchy
            .reachable_granted_authorities(&self.authentication.authorities())
            .into_iter()
            .collect()
    }

    fn role_name(&self, role: &str) -> String {
        if self.default_role_prefix.is_empty() || role.starts_with(&self.default_role_prefix) {
            role.to_string()
        } else {
            format!("{}{}", self.default_role_prefix, role)
        }
    }
}

impl SecurityExpressionOperations for SecurityExpressionRoot {
    fn has_authority(&self, authority: &str) -> bool {
        self.authorities().contains(authority)
    }

    fn has_any_authority(&self, authorities: &[String]) -> bool {
        let reachable = self.authorities();
        authorities
            .iter()
            .any(|authority| reachable.contains(authority))
    }

    fn has_role(&self, role: &str) -> bool {
        self.has_authority(&self.role_name(role))
    }

    fn has_any_role(&self, roles: &[String]) -> bool {
        let authorities = roles
            .iter()
            .map(|role| self.role_name(role))
            .collect::<Vec<_>>();
        self.has_any_authority(&authorities)
    }

    fn permit_all(&self) -> bool {
        true
    }

    fn deny_all(&self) -> bool {
        false
    }

    fn is_anonymous(&self) -> bool {
        self.trust_resolver
            .is_anonymous(Some(self.authentication.as_ref()))
    }

    fn is_authenticated(&self) -> bool {
        self.trust_resolver
            .is_authenticated(Some(self.authentication.as_ref()))
    }

    fn is_remember_me(&self) -> bool {
        self.trust_resolver
            .is_remember_me(Some(self.authentication.as_ref()))
    }

    fn is_fully_authenticated(&self) -> bool {
        self.trust_resolver
            .is_fully_authenticated(Some(self.authentication.as_ref()))
    }

    fn has_permission(&self, target: Option<&AnyValue>, permission: &str) -> bool {
        self.permission_evaluator
            .has_permission(self.authentication.as_ref(), target, permission)
    }

    fn has_permission_by_id(&self, target_id: &str, target_type: &str, permission: &str) -> bool {
        self.permission_evaluator.has_permission_by_id(
            self.authentication.as_ref(),
            target_id,
            target_type,
            permission,
        )
    }
}
