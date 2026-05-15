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
}
