//! A listener whose hooks all do nothing.

use crate::retry_listener::RetryListener;

/// A listener whose hooks all do nothing.
///
/// Every hook of [`RetryListener`] already has an empty default, so this type exists only as a
/// named base for listeners that want to be explicit about implementing nothing, which is what
/// the original class provided.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetryListenerSupport;

impl RetryListenerSupport {
    /// Creates the listener.
    pub fn new() -> Self {
        Self
    }
}

impl RetryListener for RetryListenerSupport {}

#[cfg(test)]
mod tests {
    use crate::retry_listener::RetryListener;

    use super::RetryListenerSupport;

    #[test]
    fn every_hook_is_a_no_op() {
        let listener = RetryListenerSupport::new();
        let context = crate::context::retry_context_support::RetryContextSupport::new();

        assert!(listener.open(&context));
        listener.close(&context, None);
        listener.on_success(&context, &42_u8);
        listener.on_error(
            &context,
            &crate::error::retry_error::RetryError::custom("boom"),
        );
    }
}
