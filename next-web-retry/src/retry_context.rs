//! The retry context: the state that travels through the attempts of one retry block.

use std::{any::Any, sync::Arc};

use dashmap::DashMap;
use next_web_core::any::AnyValue;

use crate::error::retry_error::RetryError;

/// The attribute names the retry machinery stores in a [`RetryContext`].
///
/// They are exposed so listeners and policies can inspect a retry without depending on
/// string literals.
pub mod retry_context_constants {
    /// The reporting name of the retry block.
    pub const NAME: &str = "context.name";
    /// The state key of a stateful retry.
    pub const STATE_KEY: &str = "context.state";
    /// Set once the context has been closed.
    pub const CLOSED: &str = "context.closed";
    /// Set when the recovery path was taken.
    pub const RECOVERED: &str = "context.recovered";
    /// Set when the retry was exhausted.
    pub const EXHAUSTED: &str = "context.exhausted";
    /// Set when a retry block did not finish normally, for example because the callback
    /// panicked or the caller cancelled the operation.
    pub const ABORTED: &str = "context.aborted";
    /// Set when the block ended with a terminated-retry failure: a listener vetoed the retry, or
    /// the failure could not be registered with the policy.
    pub const TERMINATED: &str = "context.terminated";
    /// The time the finished block took, published by the template before the close hooks run.
    ///
    /// The value is a `Duration` stored as a boxed object. It lives in the context so a listener
    /// can report it, but the template owns it: it is measured around the block and removed after
    /// the hooks ran, so a reused context never reports a stale duration.
    pub const ELAPSED: &str = "context.elapsed";
    /// Set when the last error must not be passed to a recoverer.
    pub const NO_RECOVERY: &str = "context.no-recovery";
    /// The maximum number of attempts the active policy reports.
    pub const MAX_ATTEMPTS: &str = "context.max-attempts";
    /// Marks a context that must never be closed, such as a circuit breaker context.
    pub const GLOBAL_STATE: &str = "state.global";
}

/// Low-level access to an ongoing retry operation.
///
/// A context travels through every attempt of a retry block. Policies read the attempt
/// count and the last error from it, listeners decorate it, and callbacks store their own
/// bookkeeping in it. The framework itself only requires the methods below; everything
/// else is an attribute.
///
/// Implementations must be safe to share between tasks, so the trait requires
/// [`Send`], [`Sync`] and [`Any`] (the latter lets policies downcast to their own
/// context type).
pub trait RetryContext
where
    Self: Send + Sync,
    Self: Any,
    Self: SyncAttributeAccessor,
{
    /// Signals that no further attempt should be made.
    fn set_exhausted_only(&self);

    /// Returns whether [`Self::set_exhausted_only`] was called.
    fn is_exhausted_only(&self) -> bool;

    /// Returns the parent context when retry blocks are nested.
    fn get_parent(&self) -> Option<&dyn RetryContext>;

    /// Returns the parent context when retry blocks are nested.
    ///
    /// Policies that create child contexts need an owned handle to pass down, which is what
    /// this method provides. The default returns `None`, so a context implementation that
    /// does not track a parent simply produces flat retry blocks.
    fn get_parent_shared(&self) -> Option<Arc<dyn RetryContext>> {
        None
    }

    /// Returns the number of failed attempts recorded so far.
    fn get_retry_count(&self) -> u16;

    /// Returns the error that caused the current attempt to fail.
    fn get_last_error(&self) -> Option<RetryError>;

    /// Returns the reporting name of this retry block, when one was set.
    fn get_name(&self) -> Option<String> {
        self.get_attribute(retry_context_constants::NAME)
            .and_then(|value| value.as_string())
    }

    /// Returns whether the context has been closed.
    fn is_closed(&self) -> bool {
        self.flag(retry_context_constants::CLOSED)
    }

    /// Returns whether the retry block was exhausted.
    fn is_exhausted(&self) -> bool {
        self.flag(retry_context_constants::EXHAUSTED)
    }

    /// Returns whether the recovery path produced the result.
    fn is_recovered(&self) -> bool {
        self.flag(retry_context_constants::RECOVERED)
    }

    /// Returns whether recovery is disabled for the last error.
    fn is_no_recovery(&self) -> bool {
        self.flag(retry_context_constants::NO_RECOVERY)
    }

    /// Returns the maximum number of attempts the active policy reports.
    fn get_max_attempts(&self) -> Option<u16> {
        self.get_attribute(retry_context_constants::MAX_ATTEMPTS)
            .and_then(|value| value.as_u64())
            .and_then(|value| u16::try_from(value).ok())
    }

    /// Reads a boolean attribute, treating a missing or non-boolean value as `false`.
    fn flag(&self, name: &str) -> bool {
        self.get_attribute(name)
            .and_then(|value| value.as_boolean())
            .unwrap_or_default()
    }
}

/// Attribute accessors that can be used through a shared reference.
///
/// The standard attribute accessor mutates through `&mut self`; a retry context is shared
/// between attempts and tasks, so its attributes are synchronised internally instead.
pub trait SyncAttributeAccessor
where
    Self: Send + Sync,
{
    /// Returns whether an attribute with this name exists.
    fn has_attribute(&self, name: &str) -> bool;

    /// Adds or replaces an attribute.
    fn set_attribute(&self, name: &str, value: AnyValue);

    /// Removes an attribute, returning the previous value.
    fn remove_attribute(&self, name: &str) -> Option<AnyValue>;

    /// Returns a clone of the attribute value.
    fn get_attribute(&self, name: &str) -> Option<AnyValue>;
}

/// A thread-safe attribute store, the building block of every retry context.
///
/// The store is cheap to clone: clones share the same underlying map, which is what makes
/// a context usable from several tasks at once.
#[derive(Clone, Default)]
pub struct AttributeAccessorSupport {
    attributes: Arc<DashMap<String, AnyValue>>,
}

impl AttributeAccessorSupport {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SyncAttributeAccessor for AttributeAccessorSupport {
    fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        self.attributes.insert(name.to_owned(), value);
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attributes.remove(name).map(|(_, value)| value)
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attributes.get(name).map(|value| value.clone())
    }
}

/// Implements [`RetryContext`] and [`SyncAttributeAccessor`] for a type that wraps a
/// `context_support: RetryContextSupport` field.
///
/// Policies create their own context types to hold policy-specific state. Delegating the
/// trait methods by hand is repetitive, so this macro emits the boilerplate.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone)]
/// struct MyContext {
///     context_support: RetryContextSupport,
///     deadline: std::time::Instant,
/// }
///
/// impl_retry_context!(MyContext);
/// ```
#[macro_export]
macro_rules! impl_retry_context {
    ($StructName:ident) => {
        impl $crate::retry_context::SyncAttributeAccessor for $StructName {
            fn has_attribute(&self, name: &str) -> bool {
                self.context_support.has_attribute(name)
            }

            fn set_attribute(&self, name: &str, value: next_web_core::any::AnyValue) {
                self.context_support.set_attribute(name, value)
            }

            fn remove_attribute(&self, name: &str) -> Option<next_web_core::any::AnyValue> {
                self.context_support.remove_attribute(name)
            }

            fn get_attribute(&self, name: &str) -> Option<next_web_core::any::AnyValue> {
                self.context_support.get_attribute(name)
            }
        }

        impl $crate::retry_context::RetryContext for $StructName {
            fn get_parent_shared(
                &self,
            ) -> Option<std::sync::Arc<dyn $crate::retry_context::RetryContext>> {
                self.context_support.get_parent_shared()
            }

            fn set_exhausted_only(&self) {
                self.context_support.set_exhausted_only()
            }

            fn is_exhausted_only(&self) -> bool {
                self.context_support.is_exhausted_only()
            }

            fn get_parent(&self) -> Option<&dyn $crate::retry_context::RetryContext> {
                self.context_support.get_parent()
            }

            fn get_retry_count(&self) -> u16 {
                self.context_support.get_retry_count()
            }

            fn get_last_error(&self) -> Option<$crate::error::retry_error::RetryError> {
                self.context_support.get_last_error()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_accessor_stores_reads_and_removes_values() {
        let attributes = AttributeAccessorSupport::default();

        attributes.set_attribute("answer", 42u64.into());

        assert!(attributes.has_attribute("answer"));
        assert_eq!(
            attributes
                .get_attribute("answer")
                .and_then(|value| value.as_u64()),
            Some(42)
        );
        assert_eq!(
            attributes
                .remove_attribute("answer")
                .and_then(|value| value.as_u64()),
            Some(42)
        );
        assert!(!attributes.has_attribute("answer"));
    }

    #[test]
    fn clones_share_the_same_attribute_map() {
        let attributes = AttributeAccessorSupport::default();
        let clone = attributes.clone();

        clone.set_attribute("shared", true.into());

        assert!(attributes.has_attribute("shared"));
    }
}
