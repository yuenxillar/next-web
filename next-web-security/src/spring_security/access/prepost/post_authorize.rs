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
}
