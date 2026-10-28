//! Recognises call arguments that have never been processed before.

use next_web_core::any::AnyValue;

/// Decides whether the arguments of a call have never been processed before.
///
/// A stateful retry can skip the cache lookup when the item is known to be new. When the
/// answer is indeterminate the safe value is `false`, because a wrong `true` would start a
/// retry over while the stored context is still valid.
pub trait NewMethodArgumentsIdentifier
where
    Self: Send + Sync,
{
    /// Returns whether the arguments are known to be new.
    fn is_new(&self, arguments: &[AnyValue]) -> bool;
}

impl<F> NewMethodArgumentsIdentifier for F
where
    F: Fn(&[AnyValue]) -> bool + Send + Sync,
{
    fn is_new(&self, arguments: &[AnyValue]) -> bool {
        self(arguments)
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::NewMethodArgumentsIdentifier;

    #[test]
    fn a_closure_decides_whether_arguments_are_new() {
        let identifier = |arguments: &[AnyValue]| arguments.is_empty();

        assert!(identifier.is_new(&[]));
        assert!(!identifier.is_new(&[AnyValue::from("seen")]));
    }
}
