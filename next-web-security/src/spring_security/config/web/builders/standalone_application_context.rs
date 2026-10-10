//! An application context for a configuration that is assembled on its own.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::{self},
};

use next_web_context::{
    ApplicationContext, ApplicationEvent, ApplicationEventPublisher, InstanceClone, Locale,
    MessageSource, MessageSourceResolvable, NoSuchMessageError,
};
use next_web_singletons::factory::support::Key;

/// The singleton a standalone context holds.
struct StoredSingleton {
    instance: Box<dyn Any + Send + Sync>,
    clone: Option<InstanceClone>,
}

/// An [`ApplicationContext`] that holds the singletons that were inserted into
/// it.
///
/// The context of the application is shared with the security builders by the
/// web layer, and a configuration that is assembled directly (such as a test)
/// is not given one. The builders then use this context, which holds the values
/// that the configuration inserts and resolves nothing else, so a configurer
/// that looks a value up falls back to its own default instead of failing.
#[derive(Default)]
pub struct StandaloneApplicationContext {
    singletons: HashMap<Key, StoredSingleton>,
}

impl std::fmt::Debug for StandaloneApplicationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandaloneApplicationContext")
            .field("singletons", &self.singletons.len())
            .finish()
    }
}

#[next_web_core::async_trait]
impl ApplicationContext for StandaloneApplicationContext {
    fn id(&self) -> &str {
        "standalone"
    }

    fn application_name(&self) -> &str {
        "standalone"
    }

    fn startup_date(&self) -> i64 {
        0
    }

    fn insert_singleton_boxed(
        &mut self,
        key: Key,
        instance: Box<dyn Any + Send + Sync>,
        clone: Option<InstanceClone>,
    ) {
        self.singletons
            .insert(key, StoredSingleton { instance, clone });
    }

    fn contains_singleton_boxed(&self, key: &Key) -> bool {
        self.singletons.contains_key(key)
    }

    fn get_singleton_boxed(&self, key: &Key) -> Option<&(dyn Any + Send + Sync)> {
        self.singletons
            .get(key)
            .map(|singleton| singleton.instance.as_ref())
    }

    fn get_singleton_boxed_mut(&mut self, key: &Key) -> Option<&mut (dyn Any + Send + Sync)> {
        self.singletons
            .get_mut(key)
            .map(|singleton| singleton.instance.as_mut())
    }

    fn remove_singleton_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        self.singletons
            .remove(key)
            .map(|singleton| singleton.instance)
    }

    fn resolve_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        let singleton = self.singletons.get(key)?;
        let clone = singleton.clone.clone()?;

        Some(clone(singleton.instance.as_ref()))
    }

    async fn resolve_boxed_async(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        // The context has no provider, so the singletons it holds are the ones
        // that can be produced and producing them does not await anything.
        self.resolve_boxed(key)
    }

    fn resolve_all_of_type(&mut self, ty: TypeId) -> Vec<Box<dyn Any + Send + Sync>> {
        let keys = self.keys_of_type(ty);

        keys.iter()
            .filter_map(|key| self.resolve_boxed(key))
            .collect()
    }

    fn keys_of_type(&self, ty: TypeId) -> Vec<Key> {
        let mut keys: Vec<Key> = self
            .singletons
            .keys()
            .filter(|key| key.ty.id == ty)
            .cloned()
            .collect();
        keys.sort();

        keys
    }
}

impl ApplicationEventPublisher for StandaloneApplicationContext {
    fn publish_event(
        &self,
        _event: Box<dyn ApplicationEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        Ok(())
    }
}

impl MessageSource for StandaloneApplicationContext {
    fn message_or_default(
        &self,
        _code: &str,
        _args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        _locale: Option<&Locale>,
    ) -> Option<String> {
        default_message.map(ToString::to_string)
    }

    fn message(
        &self,
        code: &str,
        _args: &[&dyn fmt::Display],
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        Err(NoSuchMessageError::new(code, locale))
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        if let Some(default_message) = resolvable.default_message() {
            return Ok(default_message.to_string());
        }

        let code = resolvable
            .codes()
            .and_then(|codes| codes.first())
            .map(String::as_str)
            .unwrap_or("<unknown>");

        Err(NoSuchMessageError::new(code, locale))
    }
}

impl fmt::Display for StandaloneApplicationContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StandaloneApplicationContext")
    }
}
