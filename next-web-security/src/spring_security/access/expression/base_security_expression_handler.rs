use std::sync::Arc;

use crate::{
    access::{
        expression::{
            deny_all_permission_evaluator::DenyAllPermissionEvaluator,
            security_expression_handler::SecurityExpressionHandler,
            security_expression_operations::SecurityExpressionOperations,
        },
        hierarchicalroles::{role_hierarchy::RoleHierarchy, NullRoleHierarchy},
        permission_evaluator::PermissionEvaluator,
    },
    authorization::{AuthenticationTrustResolver, DefaultAuthenticationTrustResolver},
    core::Authentication,
};

pub struct AbstractSecurityExpressionHandler<T> {
    permission_evaluator: Arc<dyn PermissionEvaluator>,
    role_hierarchy: Arc<dyn RoleHierarchy>,
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
    _marker: std::marker::PhantomData<T>,
}

pub struct EvaluationContext<T> {
    pub root: Box<dyn SecurityExpressionOperations>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> EvaluationContext<T> {
    pub fn root_object(&self) -> &dyn SecurityExpressionOperations {
        self.root.as_ref()
    }
}

impl<T> AbstractSecurityExpressionHandler<T> {
    pub fn new() -> Self {
        Self {
            permission_evaluator: Arc::new(DenyAllPermissionEvaluator),
            role_hierarchy: Arc::new(NullRoleHierarchy),
            trust_resolver: Arc::new(DefaultAuthenticationTrustResolver::default()),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn permission_evaluator(&self) -> Arc<dyn PermissionEvaluator> {
        self.permission_evaluator.clone()
    }

    pub fn set_permission_evaluator(&mut self, permission_evaluator: Arc<dyn PermissionEvaluator>) {
        self.permission_evaluator = permission_evaluator;
    }

    pub fn role_hierarchy(&self) -> Arc<dyn RoleHierarchy> {
        self.role_hierarchy.clone()
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    pub fn trust_resolver(&self) -> Arc<dyn AuthenticationTrustResolver> {
        self.trust_resolver.clone()
    }

    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }
}

impl<T> Default for AbstractSecurityExpressionHandler<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + Sync + 'static> SecurityExpressionHandler<T>
    for AbstractSecurityExpressionHandler<T>
{
    fn create_security_expression_root(
        &self,
        _authentication: Option<Arc<dyn Authentication>>,
        _invocation: T,
    ) -> Box<dyn SecurityExpressionOperations> {
        panic!("create_security_expression_root must be overridden by concrete implementations")
    }
}
