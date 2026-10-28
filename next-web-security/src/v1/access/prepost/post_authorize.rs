/// Rust equivalent of Spring Security's `@PostAuthorize` annotation.
///
/// Specifies a method access-control expression evaluated after method invocation.
/// In Java: `@PostAuthorize("returnObject.owner == authentication.name")`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostAuthorize {
    pub expression: String,
}

impl PostAuthorize {
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

impl super::PrePostExpressionAttribute for PostAuthorize {
    fn expression(&self) -> &str {
        &self.expression
    }
}
