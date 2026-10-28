/// Rust equivalent of Spring Security's `@PreFilter` annotation.
///
/// Specifies a method filtering expression evaluated before method invocation.
/// The `filter_target` names the parameter to filter.
/// In Java: `@PreFilter(value = "filterObject.owner == authentication.name", filterTarget = "list")`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreFilter {
    pub expression: String,
    pub filter_target: String,
}

impl PreFilter {
    pub fn new(expression: impl Into<String>, filter_target: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            filter_target: filter_target.into(),
        }
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }
    pub fn value(&self) -> &str {
        &self.expression
    }
    pub fn filter_target(&self) -> &str {
        &self.filter_target
    }
    pub fn set_expression(&mut self, expression: impl Into<String>) {
        self.expression = expression.into();
    }
    pub fn set_filter_target(&mut self, target: impl Into<String>) {
        self.filter_target = target.into();
    }

    pub fn has_filter_target(&self) -> bool {
        !self.filter_target.trim().is_empty()
    }
}

impl super::PrePostExpressionAttribute for PreFilter {
    fn expression(&self) -> &str {
        &self.expression
    }
}
