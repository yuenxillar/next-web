/// Rust equivalent of Spring Security's `@PostFilter` annotation.
///
/// Specifies a method filtering expression evaluated after method invocation.
/// In Java: `@PostFilter("filterObject.owner == authentication.name")`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostFilter {
    pub expression: String,
}

impl PostFilter {
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

impl super::PrePostExpressionAttribute for PostFilter {
    fn expression(&self) -> &str {
        &self.expression
    }
}
