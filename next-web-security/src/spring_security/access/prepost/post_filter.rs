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
}
