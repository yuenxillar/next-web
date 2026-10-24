//! The description of one intercepted call.

use next_web_core::any::AnyValue;

/// One intercepted call: what was called and with which arguments.
///
/// The original API intercepts a reflective method invocation. Rust has no reflection, so an
/// interceptor receives this description plus the closure that performs the call. The
/// description is what a key generator, a recoverer and a listener inspect, and it is stored
/// in the retry context so later hooks can see it.
#[derive(Clone, Debug, Default)]
pub struct MethodInvocation {
    method: String,
    label: Option<String>,
    arguments: Vec<AnyValue>,
}

impl MethodInvocation {
    /// Creates a description for a call.
    pub fn new(method: impl Into<String>, arguments: Vec<AnyValue>) -> Self {
        Self {
            method: method.into(),
            label: None,
            arguments,
        }
    }

    /// Creates a description without arguments.
    pub fn without_arguments(method: impl Into<String>) -> Self {
        Self::new(method, Vec::new())
    }

    /// Sets the reporting label.
    ///
    /// The label defaults to the method name; it is what statistics and logs group retries by.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns the reporting label, falling back to the method name.
    pub fn label(&self) -> &str {
        self.label.as_deref().unwrap_or(self.method.as_str())
    }

    /// Returns the name of the intercepted call.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Returns the arguments of the call.
    pub fn arguments(&self) -> &[AnyValue] {
        &self.arguments
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::MethodInvocation;

    #[test]
    fn the_label_falls_back_to_the_method_name() {
        let invocation = MethodInvocation::new("Service::call", vec![AnyValue::from(1_u64)]);

        assert_eq!(invocation.label(), "Service::call");
        assert_eq!(invocation.arguments().len(), 1);
    }

    #[test]
    fn an_explicit_label_wins() {
        let invocation = MethodInvocation::without_arguments("Service::call").with_label("orders");

        assert_eq!(invocation.label(), "orders");
        assert_eq!(invocation.method(), "Service::call");
    }
}
