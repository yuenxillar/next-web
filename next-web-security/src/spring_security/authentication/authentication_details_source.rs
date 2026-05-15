use next_web_core::anys::any_value::AnyValue;

pub trait AuthenticationDetailsSource<C>: Send + Sync {
    fn build_details(&self, context: &C) -> AnyValue;
}

#[derive(Clone, Debug, Default)]
pub struct NullAuthenticationDetailsSource;

impl<C> AuthenticationDetailsSource<C> for NullAuthenticationDetailsSource {
    fn build_details(&self, _context: &C) -> AnyValue {
        AnyValue::Null
    }
}
