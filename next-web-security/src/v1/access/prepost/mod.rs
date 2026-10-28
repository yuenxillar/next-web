pub mod post_authorize;
pub mod post_filter;
pub mod pre_authorize;
pub mod pre_filter;

/// Common contract for pre/post authorization metadata.
pub trait PrePostExpressionAttribute {
    fn expression(&self) -> &str;

    fn validate(&self) -> Result<(), &'static str> {
        if self.expression().trim().is_empty() {
            Err("authorization expression must not be empty")
        } else {
            Ok(())
        }
    }
}
