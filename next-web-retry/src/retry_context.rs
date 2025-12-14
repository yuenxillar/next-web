use std::{any::Any, collections::BTreeMap, sync::Arc};

use next_web_core::{DynClone, anys::any_value::AnyValue};
use tokio::sync::{Mutex, RwLock};

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
    Self: SyncAttributeAccessor + DynClone,
{
    fn set_exhausted_only(&self);

    fn is_exhausted_only(&self) -> bool;

    fn get_parent(&self) -> Option<&dyn RetryContext>;

    fn get_retry_count(&self) -> u16;

    fn get_last_error(&self) -> Option<RetryError>;
}

next_web_core::clone_trait_object!(RetryContext);

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
    attributes: Arc<Mutex<BTreeMap<String, AnyValue>>>,
}

impl SyncAttributeAccessor for AttributeAccessorSupport {
    fn has_attribute(&self, name: &str) -> bool {
        self.attributes
            .try_lock()
            .map(|m| m.contains_key(name))
            .unwrap_or_default()
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        self.attributes
            .try_lock()
            .map(|mut m| m.insert(name.to_string(), value)).ok();
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attributes
            .try_lock()
            .map(|mut m| m.remove(name))
            .unwrap_or_default()
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attributes.try_lock().map(|m| m.get(name).cloned()).unwrap_or_default()        
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

            fn get_attribute(&self, name: &str) -> Option<next_web_core::anys::any_value::AnyValue> {
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
