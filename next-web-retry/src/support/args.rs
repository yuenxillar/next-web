//! The arguments of an intercepted call, exposed to listeners.

use next_web_core::any::AnyValue;

/// The arguments of an intercepted call, exposed to listeners and expressions.
///
/// The original API wrapped a Java `Object[]`; the Rust counterpart wraps a vector of
/// [`AnyValue`], which is what an interceptor can capture without the callback being generic
/// over its argument types.
#[derive(Clone, Debug, Default)]
pub struct Args {
    args: Vec<AnyValue>,
}

impl Args {
    /// Creates an argument list.
    pub fn new(args: Vec<AnyValue>) -> Self {
        Self { args }
    }

    /// Creates an empty argument list.
    ///
    /// The original exposes a shared instance with one hundred empty slots; a vector makes an
    /// empty list cheaper and clearer, so this constructor replaces it.
    pub fn no_args() -> Self {
        Self::default()
    }

    /// Returns the arguments.
    pub fn get_args(&self) -> &[AnyValue] {
        &self.args
    }

    /// Returns one argument by index.
    pub fn get(&self, index: usize) -> Option<&AnyValue> {
        self.args.get(index)
    }

    /// Returns the number of arguments.
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Returns whether there are no arguments.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

impl From<Vec<AnyValue>> for Args {
    fn from(args: Vec<AnyValue>) -> Self {
        Self::new(args)
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::Args;

    #[test]
    fn arguments_are_indexed_in_order() {
        let args = Args::new(vec![AnyValue::from("first"), AnyValue::from(2_u64)]);

        assert_eq!(args.len(), 2);
        assert_eq!(args.get(0).and_then(|value| value.as_str()), Some("first"));
        assert_eq!(args.get(1).and_then(|value| value.as_u64()), Some(2));
        assert!(args.get(2).is_none());
    }

    #[test]
    fn the_empty_list_has_no_arguments() {
        let args = Args::no_args();

        assert!(args.is_empty());
        assert!(args.get_args().is_empty());
    }
}
