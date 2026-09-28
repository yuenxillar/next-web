//! The context the tests of the crate resolve their singletons from.
//!
//! The context is a minimal in-memory one: it holds the instances that were
//! inserted into it, hands them out the way the accessors of the crate expect,
//! and resolves a key by copying the instance the key holds. It has no
//! providers, so a key it does not hold cannot be resolved.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

use async_trait::async_trait;
use next_web_singletons::factory::support::Key;

use crate::{
    ApplicationContext, ApplicationEvent, ApplicationEventPublisher, InstanceClone, Locale,
    MessageSource, MessageSourceResolvable, NoSuchMessageError,
};

/// Minimal in-memory context used to exercise the helpers.
#[derive(Default)]
pub(crate) struct FakeContext {
    instances: HashMap<Key, (Box<dyn Any + Send + Sync>, Option<InstanceClone>)>,
}

impl FakeContext {
    fn find(&self, key: &Key) -> Option<&(Box<dyn Any + Send + Sync>, Option<InstanceClone>)> {
        self.instances.get(key)
    }
}

#[async_trait]
impl ApplicationContext for FakeContext {
    fn id(&self) -> &str {
        "fake"
    }

    fn application_name(&self) -> &str {
        "fake"
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
        self.instances.insert(key, (instance, clone));
    }

    fn contains_singleton_boxed(&self, key: &Key) -> bool {
        self.instances.contains_key(key)
    }

    fn get_singleton_boxed(&self, key: &Key) -> Option<&(dyn Any + Send + Sync)> {
        self.find(key).map(|(instance, _)| instance.as_ref())
    }

    fn get_singleton_boxed_mut(&mut self, key: &Key) -> Option<&mut (dyn Any + Send + Sync)> {
        self.instances
            .get_mut(key)
            .map(|(instance, _)| instance.as_mut())
    }

    fn remove_singleton_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        self.instances.remove(key).map(|(instance, _)| instance)
    }

    fn resolve_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        let clone = self.find(key)?.1.clone()?;
        Some(clone(self.instances.get(key)?.0.as_ref()))
    }

    async fn resolve_boxed_async(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        // The context has no provider, so the instances it holds are the ones
        // that can be resolved, and resolving them does not await anything.
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
            .instances
            .keys()
            .filter(|key| key.ty.id == ty)
            .cloned()
            .collect();
        keys.sort();
        keys
    }
}

impl ApplicationEventPublisher for FakeContext {
    fn publish_event(
        &self,
        _event: Box<dyn ApplicationEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

impl MessageSource for FakeContext {
    fn message_or_default(
        &self,
        _code: &str,
        _args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        _locale: Option<&Locale>,
    ) -> Option<String> {
        default_message.map(|message| message.to_owned())
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
        let code = resolvable
            .codes()
            .and_then(|codes| codes.last())
            .map(String::as_str)
            .unwrap_or_default();

        Err(NoSuchMessageError::new(code, locale))
    }
}

impl Debug for FakeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeContext")
            .field("instances", &"none")
            .finish()
    }
}
