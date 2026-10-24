//! A key generator that always returns the same key.

use next_web_core::any::AnyValue;

use super::method_arguments_key_generator::MethodArgumentsKeyGenerator;

/// A key generator that always returns the same key.
///
/// Use it for a retry block that processes one item at a time: the retry is then tracked under
/// a constant name instead of the call arguments.
#[derive(Clone, Debug)]
pub struct FixedKeyGenerator {
    label: String,
}

impl FixedKeyGenerator {
    /// Creates a generator that always returns `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    /// Returns the constant key.
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl MethodArgumentsKeyGenerator for FixedKeyGenerator {
    fn get_key(&self, _arguments: &[AnyValue]) -> Option<String> {
        Some(self.label.clone())
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::FixedKeyGenerator;
    use crate::interceptor::method_arguments_key_generator::MethodArgumentsKeyGenerator;

    #[test]
    fn the_key_ignores_the_arguments() {
        let generator = FixedKeyGenerator::new("single-item");

        assert_eq!(
            generator.get_key(&[AnyValue::from(1_u64)]),
            Some("single-item".to_owned())
        );
        assert_eq!(generator.get_key(&[]), Some("single-item".to_owned()));
    }
}
