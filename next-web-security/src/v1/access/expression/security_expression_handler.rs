use std::sync::Arc;

use crate::core::Authentication;

use super::security_expression_operations::SecurityExpressionOperations;

pub trait SecurityExpressionHandler<T>: Send + Sync {
    fn create_security_expression_root(
        &self,
        authentication: Option<Arc<dyn Authentication>>,
        invocation: T,
    ) -> Box<dyn SecurityExpressionOperations>;
}
