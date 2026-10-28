use crate::access::expression::method::method_security_expression_operations::MethodSecurityExpressionOperations;

use super::method_security_expression_handler::MethodInvocation;

pub struct MethodSecurityEvaluationContext {
    root_object: Box<dyn MethodSecurityExpressionOperations>,
    method_invocation: MethodInvocation,
}

impl MethodSecurityEvaluationContext {
    pub fn new(
        root_object: Box<dyn MethodSecurityExpressionOperations>,
        method_invocation: MethodInvocation,
    ) -> Self {
        Self {
            root_object,
            method_invocation,
        }
    }

    pub fn root_object(&self) -> &dyn MethodSecurityExpressionOperations {
        self.root_object.as_ref()
    }

    pub fn method_invocation(&self) -> &MethodInvocation {
        &self.method_invocation
    }
}
