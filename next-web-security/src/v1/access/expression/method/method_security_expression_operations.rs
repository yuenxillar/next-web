use crate::access::expression::security_expression_operations::SecurityExpressionOperations;

pub trait MethodSecurityExpressionOperations: SecurityExpressionOperations {
    fn set_filter_object(&mut self, filter_object: Option<Box<dyn std::any::Any + Send + Sync>>);
    fn get_filter_object(&self) -> Option<&(dyn std::any::Any + Send + Sync)>;
    fn set_return_object(&mut self, return_object: Option<Box<dyn std::any::Any + Send + Sync>>);
    fn get_return_object(&self) -> Option<&(dyn std::any::Any + Send + Sync)>;
    fn get_this(&self) -> Option<&(dyn std::any::Any + Send + Sync)>;
}
