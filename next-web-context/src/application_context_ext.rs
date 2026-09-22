//! Type safe helpers built on top of [`ApplicationContext`].
//!
//! [`ApplicationContext`] itself is object safe and therefore works with type
//! erased instances. The methods of [`ApplicationContextExt`] add the generic
//! API that generated code and application code use, and they are implemented
//! for every context, including `dyn ApplicationContext`:
//!
//! ```
//! use next_web_context::{ApplicationContext, ApplicationContextExt};
//!
//! fn configure(context: &mut dyn ApplicationContext) {
//!     // `ApplicationContextExt` has to be in scope for this call.
//!     let _ = context.get_single_option::<u32>();
//! }
//! ```

use std::any::{Any, TypeId};
use std::sync::Arc;
use std::borrow::Cow;

use next_web_singletons::factory::support::Key;

use crate::application_context::{InstanceClone, default_singleton_name};
use crate::ApplicationContext;

/// Type safe access to the singletons of an [`ApplicationContext`].
///
/// The trait is blanket implemented, so it is enough to import it to call its
/// methods on any context:
///
/// ```
/// use next_web_context::{ApplicationContext, ApplicationContextExt};
///
/// fn configure(context: &mut dyn ApplicationContext) {
///     let _ = context.get_single_option::<u32>();
/// }
/// ```
///
/// # Names
///
/// Every method has three variants, following the same convention as the rest
/// of the framework:
///
/// - no suffix: the instance is registered under the empty name `""`,
/// - `with_name`: the instance is registered under the given name,
/// - `with_default_name`: the instance is registered under the name derived
///   from its type by [`default_singleton_name`].
pub trait ApplicationContextExt: ApplicationContext {
    /// Inserts `instance` under the empty name `""`.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance to insert.
    fn insert_singleton<T>(&mut self, instance: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.insert_singleton_with_name(instance, "");
    }

    /// Inserts `instance` under the given name.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance to insert.
    /// * `name` - The name to register the instance under.
    fn insert_singleton_with_name<T>(&mut self, instance: T, name: impl Into<Cow<'static, str>>)
    where
        T: Clone + Send + Sync + 'static,
    {
        /// Clones the instance an [`ApplicationContext`] stored.
        fn clone_instance<T>(instance: &(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync>
        where
            T: Clone + Send + Sync + 'static,
        {
            let instance = instance
                .downcast_ref::<T>()
                .expect("the context stores an instance of its key type");

            Box::new(instance.clone())
        }

        let key = Key::new::<T>(name.into());
        let clone: InstanceClone = Arc::new(clone_instance::<T>);
        self.insert_singleton_boxed(key, Box::new(instance), Some(clone));
    }

    /// Inserts `instance` under the name derived from its type.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance to insert.
    fn insert_singleton_with_default_name<T>(&mut self, instance: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.insert_singleton_with_name(instance, default_singleton_name::<T>());
    }

    /// Returns whether an instance is registered under the empty name `""`.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn contains_single<T>(&self) -> bool
    where
        T: 'static,
    {
        self.contains_single_with_name::<T>("")
    }

    /// Returns whether an instance is registered under the given name.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn contains_single_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> bool
    where
        T: 'static,
    {
        self.contains_singleton(&Key::new::<T>(name.into()))
    }

    /// Returns whether an instance is registered under the name derived from its
    /// type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn contains_single_with_default_name<T>(&self) -> bool
    where
        T: 'static,
    {
        self.contains_single_with_name::<T>(default_singleton_name::<T>())
    }

    /// Returns a reference to the instance registered under the empty name `""`.
    ///
    /// # Panics
    ///
    /// Panics when no instance is registered for the type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    #[track_caller]
    fn get_single<T>(&self) -> &T
    where
        T: 'static,
    {
        self.get_single_with_name::<T>("")
    }

    /// Returns a reference to the instance registered under the given name.
    ///
    /// # Panics
    ///
    /// Panics when no instance is registered for the type and name.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    #[track_caller]
    fn get_single_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> &T
    where
        T: 'static,
    {
        let key = Key::new::<T>(name.into());
        self.get_singleton_boxed(&key)
            .and_then(|instance| instance.downcast_ref::<T>())
            .unwrap_or_else(|| panic!("no instance is registered for: {key:?}"))
    }

    /// Returns a reference to the instance registered under the name derived
    /// from its type, when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn get_single_with_default_name<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.get_single_option_with_name::<T>(default_singleton_name::<T>())
    }

    /// Returns a reference to the instance registered under the empty name `""`,
    /// when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn get_single_option<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.get_single_option_with_name::<T>("")
    }

    /// Returns a reference to the instance registered under the given name, when
    /// there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn get_single_option_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> Option<&T>
    where
        T: 'static,
    {
        let key = Key::new::<T>(name.into());
        self.get_singleton_boxed(&key)
            .and_then(|instance| instance.downcast_ref::<T>())
    }

    /// Returns a mutable reference to the instance registered under the empty
    /// name `""`, when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn get_single_option_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.get_single_option_mut_with_name::<T>("")
    }

    /// Returns a mutable reference to the instance registered under the given
    /// name, when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn get_single_option_mut_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>) -> Option<&mut T>
    where
        T: 'static,
    {
        let key = Key::new::<T>(name.into());
        self.get_singleton_boxed_mut(&key)
            .and_then(|instance| instance.downcast_mut::<T>())
    }

    /// Resolves the instance registered under the empty name `""`.
    ///
    /// The context returns a stored instance when it has one, and constructs the
    /// instance through a registered provider otherwise.
    ///
    /// # Panics
    ///
    /// Panics when the instance cannot be resolved.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    #[track_caller]
    fn resolve<T>(&mut self) -> T
    where
        T: Send + Sync + 'static,
    {
        self.resolve_with_name::<T>("")
    }

    /// Resolves the instance registered under the empty name `""`, when there is
    /// one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn resolve_option<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_option_with_name::<T>("")
    }

    /// Resolves the instance registered under the given name.
    ///
    /// # Panics
    ///
    /// Panics when the instance cannot be resolved.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    #[track_caller]
    fn resolve_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>) -> T
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        self.resolve_option_boxed(&key)
            .unwrap_or_else(|| panic!("no instance could be resolved for: {key:?}"))
    }

    /// Resolves the instance registered under the given name, when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn resolve_option_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());
        self.resolve_option_boxed(&key)
    }

    /// Resolves the instance registered under the name derived from its type.
    ///
    /// # Panics
    ///
    /// Panics when the instance cannot be resolved.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    #[track_caller]
    fn resolve_with_default_name<T>(&mut self) -> T
    where
        T: Send + Sync + 'static,
    {
        self.resolve_with_name::<T>(default_singleton_name::<T>())
    }

    /// Resolves the instance registered under the name derived from its type,
    /// when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn resolve_option_with_default_name<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_option_with_name::<T>(default_singleton_name::<T>())
    }

    /// Resolves every instance of the given type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instances to resolve.
    fn resolve_by_type<T>(&mut self) -> Vec<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_all_of_type(TypeId::of::<T>())
            .into_iter()
            .filter_map(|instance| instance.downcast::<T>().ok().map(|instance| *instance))
            .collect()
    }

    /// Resolves the instance registered under the given key and downcasts it.
    ///
    /// This is the shared implementation of the `resolve*` methods.
    fn resolve_option_boxed<T>(&mut self, key: &Key) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_boxed(key)
            .map(|instance| *instance.downcast::<T>().expect(
                "the context resolved an instance whose type does not match its key",
            ))
    }
}

impl<C> ApplicationContextExt for C where C: ApplicationContext + ?Sized {}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::HashMap;
    use std::fmt;

    use crate::{
        ApplicationEvent, ApplicationEventPublisher, Locale, MessageSource,
        MessageSourceResolvable, NoSuchMessageError,
    };

    use super::*;

    /// Minimal in-memory context used to exercise the helpers.
    #[derive(Default)]
    struct FakeContext {
        instances: HashMap<Key, (Box<dyn Any + Send + Sync>, Option<InstanceClone>)>,
    }

    impl FakeContext {
        fn find(&self, key: &Key) -> Option<&(Box<dyn Any + Send + Sync>, Option<InstanceClone>)> {
            self.instances.get(key)
        }
    }

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

        fn contains_singleton(&self, key: &Key) -> bool {
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

        fn resolve_all_of_type(&mut self, ty: TypeId) -> Vec<Box<dyn Any + Send + Sync>> {
            let keys: Vec<Key> = self
                .instances
                .keys()
                .filter(|key| key.ty.id == ty)
                .cloned()
                .collect();

            keys.iter().filter_map(|key| self.resolve_boxed(key)).collect()
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
            _args: Option<&[&dyn fmt::Display]>,
            default_message: Option<&str>,
            _locale: Option<&Locale>,
        ) -> Option<String> {
            default_message.map(|message| message.to_owned())
        }

        fn message(
            &self,
            code: &str,
            _args: Option<&[&dyn fmt::Display]>,
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

    /// Type used to check the default name convention.
    #[derive(Clone)]
    struct MyService;

    #[test]
    fn inserts_and_reads_instances_by_name() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(7_u32, "answer");

        assert!(context.contains_single_with_name::<u32>("answer"));
        assert!(!context.contains_single_with_name::<u32>("missing"));
        assert_eq!(context.get_single_with_name::<u32>("answer"), &7);
    }

    #[test]
    fn resolves_owned_copies_of_instances() {
        let mut context = FakeContext::default();
        context.insert_singleton("value".to_owned());

        assert_eq!(context.resolve::<String>(), "value");
        assert_eq!(
            context.resolve_option::<String>(),
            Some("value".to_owned())
        );
    }

    #[test]
    fn resolves_unknown_instances_as_none() {
        let mut context = FakeContext::default();

        assert!(context.resolve_option::<u32>().is_none());
        assert!(context.get_single_option::<u32>().is_none());
    }

    #[test]
    fn uses_the_default_name_of_a_type() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_default_name(1_u64);
        context.insert_singleton_with_default_name(MyService);

        assert_eq!(context.get_single_with_default_name::<u64>(), Some(&1));
        assert_eq!(context.resolve_with_default_name::<u64>(), 1);
        assert!(context.contains_single_with_default_name::<MyService>());
        assert_eq!(default_singleton_name::<MyService>(), "myService");
    }

    #[test]
    fn resolves_every_instance_of_a_type() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(1_i32, "one");
        context.insert_singleton_with_name(2_i32, "two");
        context.insert_singleton_with_name("other".to_owned(), "one");

        let mut values = context.resolve_by_type::<i32>();
        values.sort();

        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn removes_instances() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(1_u8, "one");

        let key = Key::new::<u8>("one".into());
        let removed = context.remove_singleton_boxed(&key);

        assert_eq!(removed.and_then(|instance| instance.downcast::<u8>().ok()).map(|instance| *instance), Some(1));
        assert!(!context.contains_single_with_name::<u8>("one"));
    }
}



