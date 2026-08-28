use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::{
    access::{
        expression::security_expression_operations::SecurityExpressionOperations,
        expression::security_expression_root::SecurityExpressionRoot,
        hierarchicalroles::RoleHierarchy, permission_evaluator::PermissionEvaluator,
    },
    authorization::AuthenticationTrustResolver,
    core::Authentication,
};

use super::{
    method_security_expression_handler::MethodInvocation,
    method_security_expression_operations::MethodSecurityExpressionOperations,
};

pub struct MethodSecurityExpressionRoot {
    invocation: MethodInvocation,
    filter_object: Option<Box<dyn std::any::Any + Send + Sync>>,
    return_object: Option<Box<dyn std::any::Any + Send + Sync>>,

    base: SecurityExpressionRoot,
}

impl MethodSecurityExpressionRoot {
    pub fn new(authentication: Arc<dyn Authentication>, invocation: MethodInvocation) -> Self {
        Self {
            base: SecurityExpressionRoot::new(authentication),
            invocation,
            filter_object: None,
            return_object: None,
        }
    }

    pub fn set_permission_evaluator(&mut self, permission_evaluator: Arc<dyn PermissionEvaluator>) {
        self.base.set_permission_evaluator(permission_evaluator);
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.base.set_role_hierarchy(role_hierarchy);
    }

    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.base.set_trust_resolver(trust_resolver);
    }

    pub fn set_default_role_prefix(&mut self, default_role_prefix: impl Into<String>) {
        self.base.set_default_role_prefix(default_role_prefix);
    }

    pub fn set_this(&mut self, target: Option<Box<dyn std::any::Any + Send + Sync>>) {
        self.invocation.target = target;
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.base.authentication()
    }
}

impl MethodSecurityExpressionOperations for MethodSecurityExpressionRoot {
    fn set_filter_object(&mut self, filter_object: Option<Box<dyn std::any::Any + Send + Sync>>) {
        self.filter_object = filter_object;
    }

    fn get_filter_object(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
        self.filter_object.as_ref().map(|o| o.as_ref())
    }

    fn set_return_object(&mut self, return_object: Option<Box<dyn std::any::Any + Send + Sync>>) {
        self.return_object = return_object;
    }

    fn get_return_object(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
        self.return_object.as_ref().map(|o| o.as_ref())
    }

    fn get_this(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
        self.invocation.get_this()
    }
}

impl SecurityExpressionOperations for MethodSecurityExpressionRoot {
    fn has_authority(&self, authority: &str) -> bool {
        self.base.has_authority(authority)
    }

    fn has_any_authority(&self, authorities: &[String]) -> bool {
        self.base.has_any_authority(authorities)
    }

    fn has_role(&self, role: &str) -> bool {
        self.base.has_role(role)
    }

    fn has_any_role(&self, roles: &[String]) -> bool {
        self.base.has_any_role(roles)
    }

    fn permit_all(&self) -> bool {
        self.base.permit_all()
    }

    fn deny_all(&self) -> bool {
        self.base.deny_all()
    }

    fn is_anonymous(&self) -> bool {
        self.base.is_anonymous()
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn is_remember_me(&self) -> bool {
        self.base.is_remember_me()
    }

    fn is_fully_authenticated(&self) -> bool {
        self.base.is_fully_authenticated()
    }

    fn has_permission(&self, target: Option<&AnyValue>, permission: &str) -> bool {
        self.base.has_permission(target, permission)
    }

    fn has_permission_by_id(&self, target_id: &str, target_type: &str, permission: &str) -> bool {
        self.base
            .has_permission_by_id(target_id, target_type, permission)
    }
}
