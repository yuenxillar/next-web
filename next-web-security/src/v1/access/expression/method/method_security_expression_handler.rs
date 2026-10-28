use crate::access::expression::security_expression_handler::SecurityExpressionHandler;

use super::method_security_evaluation_context::MethodSecurityEvaluationContext;

pub trait MethodSecurityExpressionHandler: SecurityExpressionHandler<MethodInvocation> {
    fn filter(
        &self,
        filter_target: Option<Box<dyn std::any::Any + Send + Sync>>,
        filter_expression: &dyn Fn(&(dyn std::any::Any + Send + Sync)) -> bool,
        ctx: &MethodSecurityEvaluationContext,
    ) -> Option<Box<dyn std::any::Any + Send + Sync>>;

    fn set_return_object(
        &self,
        return_object: Option<Box<dyn std::any::Any + Send + Sync>>,
        ctx: &MethodSecurityEvaluationContext,
    );
}

pub struct MethodInvocation {
    pub method_name: String,
    pub arguments: Vec<Box<dyn std::any::Any + Send + Sync>>,
    pub target: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl MethodInvocation {
    pub fn new(
        method_name: impl Into<String>,
        arguments: Vec<Box<dyn std::any::Any + Send + Sync>>,
        target: Option<Box<dyn std::any::Any + Send + Sync>>,
    ) -> Self {
        Self {
            method_name: method_name.into(),
            arguments,
            target,
        }
    }

    pub fn get_this(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
        self.target.as_ref().map(|t| t.as_ref())
    }
}
