use std::sync::Arc;

use crate::{
    access::{
        expression::{
            base_security_expression_handler::AbstractSecurityExpressionHandler,
            method::method_security_expression_root::MethodSecurityExpressionRoot,
            security_expression_handler::SecurityExpressionHandler,
            security_expression_operations::SecurityExpressionOperations,
        },
        permission_evaluator::PermissionEvaluator,
    },
    core::Authentication,
};

use super::{
    method_security_expression_handler::{MethodInvocation, MethodSecurityExpressionHandler},
    method_security_evaluation_context::MethodSecurityEvaluationContext,
};

const DEFAULT_ROLE_PREFIX: &str = "ROLE_";

pub struct DefaultMethodSecurityExpressionHandler {
    handler: AbstractSecurityExpressionHandler<MethodInvocation>,
    default_role_prefix: String,
}

impl DefaultMethodSecurityExpressionHandler {
    pub fn new() -> Self {
        Self {
            handler: AbstractSecurityExpressionHandler::new(),
            default_role_prefix: DEFAULT_ROLE_PREFIX.to_string(),
        }
    }

    pub fn permission_evaluator(&self) -> Arc<dyn PermissionEvaluator> {
        self.handler.permission_evaluator()
    }

    pub fn set_permission_evaluator(&mut self, permission_evaluator: Arc<dyn PermissionEvaluator>) {
        self.handler.set_permission_evaluator(permission_evaluator);
    }

    pub fn default_role_prefix(&self) -> &str {
        &self.default_role_prefix
    }

    pub fn set_default_role_prefix(&mut self, default_role_prefix: impl Into<String>) {
        self.default_role_prefix = default_role_prefix.into();
    }

    fn build_security_expression_root(
        &self,
        authentication: Option<Arc<dyn Authentication>>,
        invocation: MethodInvocation,
    ) -> MethodSecurityExpressionRoot {
        let mut root = if let Some(auth) = authentication {
            MethodSecurityExpressionRoot::new(auth, invocation)
        } else {
            panic!("Authentication must not be null")
        };

        root.set_permission_evaluator(self.permission_evaluator());
        if self.default_role_prefix != DEFAULT_ROLE_PREFIX {
            root.set_default_role_prefix(self.default_role_prefix.clone());
        }
        root
    }
}

impl Default for DefaultMethodSecurityExpressionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MethodSecurityExpressionHandler for DefaultMethodSecurityExpressionHandler {
    fn filter(
        &self,
        filter_target: Option<Box<dyn std::any::Any + Send + Sync>>,
        filter_expression: &dyn Fn(&(dyn std::any::Any + Send + Sync)) -> bool,
        _ctx: &MethodSecurityEvaluationContext,
    ) -> Option<Box<dyn std::any::Any + Send + Sync>> {
        let filter_target = filter_target?;

        if let Ok(collection) =
            filter_target.downcast::<Vec<Box<dyn std::any::Any + Send + Sync>>>()
        {
            let retained: Vec<Box<dyn std::any::Any + Send + Sync>> = collection
                .into_iter()
                .filter(|obj| filter_expression(obj.as_ref()))
                .collect();
            return Some(Box::new(retained));
        }

        None
    }

    fn set_return_object(
        &self,
        _return_object: Option<Box<dyn std::any::Any + Send + Sync>>,
        _ctx: &MethodSecurityEvaluationContext,
    ) {
    }
}

impl SecurityExpressionHandler<MethodInvocation> for DefaultMethodSecurityExpressionHandler {
    fn create_security_expression_root(
        &self,
        authentication: Option<Arc<dyn Authentication>>,
        invocation: MethodInvocation,
    ) -> Box<dyn SecurityExpressionOperations> {
        Box::new(self.build_security_expression_root(authentication, invocation))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        access::expression::{
            method::{
                default_method_security_expression_handler::DefaultMethodSecurityExpressionHandler,
                method_security_expression_handler::MethodInvocation,
            },
            security_expression_handler::SecurityExpressionHandler,
        },
        core::{authority_utils::AuthorityUtils, simple_authentication::SimpleAuthentication},
    };

    #[test]
    fn test_create_expression_root() {
        let auth = Arc::new(
            SimpleAuthentication::builder()
                .principal("alice")
                .authorities(AuthorityUtils::create_authority_list(["ROLE_USER"]))
                .authenticated(true)
                .build(),
        );

        let handler = DefaultMethodSecurityExpressionHandler::new();
        let invocation = MethodInvocation::new("testMethod", vec![], None);
        let root = SecurityExpressionHandler::<MethodInvocation>::create_security_expression_root(
            &handler,
            Some(auth),
            invocation,
        );

        assert!(root.has_role("USER"));
        assert!(!root.has_role("ADMIN"));
        assert!(root.is_authenticated());
    }
}
