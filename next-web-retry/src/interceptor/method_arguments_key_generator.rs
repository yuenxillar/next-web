//! Derives the cache key of a stateful retry from call arguments.

use next_web_core::any::AnyValue;

/// Derives the cache key of a stateful retry from the arguments of a call.
///
/// The key identifies the item being processed so a later call for the same item resumes the
/// stored retry context. Returning `None` disables retrying for the call: the interceptor then
/// lets the failure propagate immediately.
pub trait MethodArgumentsKeyGenerator
where
    Self: Send + Sync,
{
    /// Returns the key for the given arguments.
    fn get_key(&self, arguments: &[AnyValue]) -> Option<String>;
}

impl<F> MethodArgumentsKeyGenerator for F
where
    F: Fn(&[AnyValue]) -> Option<String> + Send + Sync,
{
    fn get_key(&self, arguments: &[AnyValue]) -> Option<String> {
        self(arguments)
    }
}

/// Returns the arguments joined by a separator, which is enough when a call has a single
/// identifying argument.
pub fn default_key(arguments: &[AnyValue]) -> Option<String> {
    if arguments.is_empty() {
        return None;
    }

    Some(
        arguments
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
    )
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::{MethodArgumentsKeyGenerator, default_key};

    #[test]
    fn a_closure_generates_a_key() {
        let generator = |arguments: &[AnyValue]| arguments.first().map(ToString::to_string);

        assert_eq!(
            generator.get_key(&[AnyValue::from(7_u64)]),
            Some("7".to_owned())
        );
    }

    #[test]
    fn the_default_key_joins_every_argument() {
        let arguments = vec![AnyValue::from("a"), AnyValue::from(1_u64)];

        assert_eq!(default_key(&arguments), Some("a,1".to_owned()));
        assert_eq!(default_key(&[]), None);
    }
}
