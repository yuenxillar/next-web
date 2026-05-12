use std::{any::Any, sync::Arc};

use dashmap::DashMap;
use next_web_core::{DynClone, anys::any_value::AnyValue};

use crate::error::retry_error::RetryError;

pub mod retry_context_constants {
    pub const NAME: &str = "context.name";
    pub const STATE_KEY: &str = "context.state";
    pub const CLOSED: &str = "context.closed";
    pub const RECOVERED: &str = "context.recovered";
    pub const EXHAUSTED: &str = "context.exhausted";
    pub const NO_RECOVERY: &str = "context.no-recovery";
    pub const MAX_ATTEMPTS: &str = "context.max-attempts";
}
pub trait RetryContext
where
    Self: Send + Sync,
    Self: Any,
    Self: SyncAttributeAccessor,
{
    fn set_exhausted_only(&self);

    fn is_exhausted_only(&self) -> bool;

    fn get_parent(&self) -> Option<&dyn RetryContext>;

    fn get_retry_count(&self) -> u16;

    fn get_last_error(&self) -> Option<RetryError>;
}


pub trait SyncAttributeAccessor
where
    Self: Send + Sync,
{
    fn has_attribute(&self, name: &str) -> bool;

    fn set_attribute(&self, name: &str, value: AnyValue);

    fn remove_attribute(&self, name: &str) -> Option<AnyValue>;

    fn get_attribute(&self, name: &str) -> Option<AnyValue>;
}

#[derive(Clone, Default)]
pub struct AttributeAccessorSupport {
    attributes: Arc<DashMap<String, AnyValue>>,
}

impl SyncAttributeAccessor for AttributeAccessorSupport {
    fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        self.attributes.insert(name.to_string(), value);
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attributes.remove(name).map(|(_, value)| value)
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attributes.get(name).map(|value| value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_accessor_stores_reads_and_removes_values() {
        let attributes = AttributeAccessorSupport::default();

        attributes.set_attribute("answer", AnyValue::Number(42));

        assert!(attributes.has_attribute("answer"));
        assert_eq!(
            attributes
                .get_attribute("answer")
                .and_then(|value| value.as_number()),
            Some(42)
        );
        assert_eq!(
            attributes
                .remove_attribute("answer")
                .and_then(|value| value.as_number()),
            Some(42)
        );
        assert!(!attributes.has_attribute("answer"));
    }
}

#[macro_export]
macro_rules! impl_retry_context {
    ($StructName:ident) => {
        impl crate::retry_context::SyncAttributeAccessor for $StructName {
            fn has_attribute(&self, name: &str) -> bool {
                self.context_support.has_attribute(name)
            }

            fn set_attribute(&self, name: &str, value: next_web_core::anys::any_value::AnyValue) {
                self.context_support.set_attribute(name, value)
            }

            fn remove_attribute(
                &self,
                name: &str,
            ) -> Option<next_web_core::anys::any_value::AnyValue> {
                self.context_support.remove_attribute(name)
            }

            fn get_attribute(
                &self,
                name: &str,
            ) -> Option<next_web_core::anys::any_value::AnyValue> {
                self.context_support.get_attribute(name)
            }
        }

        impl crate::retry_context::RetryContext for $StructName {
            fn set_exhausted_only(&self) {
                self.context_support.set_exhausted_only()
            }

            fn is_exhausted_only(&self) -> bool {
                self.context_support.is_exhausted_only()
            }

            fn get_parent(&self) -> Option<&dyn crate::retry_context::RetryContext> {
                self.context_support.get_parent()
            }

            fn get_retry_count(&self) -> u16 {
                self.context_support.get_retry_count()
            }

            fn get_last_error(&self) -> Option<crate::error::retry_error::RetryError> {
                self.context_support.get_last_error()
            }
        }
    };
}
