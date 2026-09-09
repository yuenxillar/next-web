/// Rust equivalent of Spring Security's `@PreAuthorize` annotation.
///
/// Specifies a method access-control expression evaluated before method invocation.
/// In Java: `@PreAuthorize("hasRole('ADMIN')")`
///
/// In Rust, use this struct programmatically or with a proc-macro attribute
/// like `#[pre_authorize("has_role('ADMIN')")]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreAuthorize {
    pub expression: String,
}

impl PreAuthorize {
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
        }
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }
    pub fn value(&self) -> &str {
        &self.expression
    }
    pub fn set_expression(&mut self, expression: impl Into<String>) {
        self.expression = expression.into();
    }
}

impl super::PrePostExpressionAttribute for PreAuthorize {
    fn expression(&self) -> &str {
        &self.expression
    }
}
