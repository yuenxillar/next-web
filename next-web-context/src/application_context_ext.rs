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
//!     let _ = context.get_singleton_option::<u32>();
//! }
//! ```

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::sync::Arc;

use next_web_singletons::factory::support::Key;

use crate::ApplicationContext;
use crate::application_context::{InstanceClone, default_singleton_name};

/// Type safe access to the singletons of an [`ApplicationContext`].
///
/// The trait is blanket implemented, so it is enough to import it to call its
/// methods on any context:
///
/// ```
/// use next_web_context::{ApplicationContext, ApplicationContextExt};
///
/// fn configure(context: &mut dyn ApplicationContext) {
///     let _ = context.get_singleton_option::<u32>();
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
///
/// The type erased counterpart of a method is the `_boxed` method of
/// [`ApplicationContext`], which takes a [`Key`] instead of a type parameter.
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
    fn contains_singleton<T>(&self) -> bool
    where
        T: 'static,
    {
        self.contains_singleton_with_name::<T>("")
    }

    /// Returns whether an instance is registered under the given name.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn contains_singleton_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> bool
    where
        T: 'static,
    {
        self.contains_singleton_boxed(&Key::new::<T>(name.into()))
    }

    /// Returns whether an instance is registered under the name derived from its
    /// type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn contains_singleton_with_default_name<T>(&self) -> bool
    where
        T: 'static,
    {
        self.contains_singleton_with_name::<T>(default_singleton_name::<T>())
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
    fn get_singleton<T>(&self) -> &T
    where
        T: 'static,
    {
        self.get_singleton_with_name::<T>("")
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
    fn get_singleton_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> &T
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
    fn get_singleton_option_with_default_name<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.get_singleton_option_with_name::<T>(default_singleton_name::<T>())
    }

    /// Returns a reference to the instance registered under the empty name `""`,
    /// when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn get_singleton_option<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.get_singleton_option_with_name::<T>("")
    }

    /// Returns a reference to the instance registered under the given name, when
    /// there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn get_singleton_option_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> Option<&T>
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
    fn get_singleton_option_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.get_singleton_option_mut_with_name::<T>("")
    }

    /// Returns a mutable reference to the instance registered under the given
    /// name, when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn get_singleton_option_mut_with_name<T>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&mut T>
    where
        T: 'static,
    {
        let key = Key::new::<T>(name.into());
        self.get_singleton_boxed_mut(&key)
            .and_then(|instance| instance.downcast_mut::<T>())
    }

    /// Returns a mutable reference to the instance registered under the given
    /// name.
    ///
    /// The mutable reference is what lets an item that is built by the context
    /// modify a singleton: the instance is borrowed from the context, so the
    /// changes are the ones the context keeps.
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
    fn get_singleton_mut_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>) -> &mut T
    where
        T: 'static,
    {
        let key = Key::new::<T>(name.into());
        self.get_singleton_boxed_mut(&key)
            .and_then(|instance| instance.downcast_mut::<T>())
            .unwrap_or_else(|| panic!("no instance is registered for: {key:?}"))
    }

    /// Removes the instance registered under the empty name `""`, when the
    /// context holds it, and returns it.
    ///
    /// Only the instance is removed; the provider that created it is kept, so
    /// resolving the instance again builds a new one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn remove_singleton<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.remove_singleton_with_name::<T>("")
    }

    /// Removes the instance registered under the given name, when the context
    /// holds it, and returns it.
    ///
    /// Only the instance is removed; the provider that created it is kept, so
    /// resolving the instance again builds a new one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn remove_singleton_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        self.remove_singleton_boxed(&key).map(|instance| {
            *instance
                .downcast::<T>()
                .expect("the context holds an instance of the type of its key")
        })
    }

    /// Removes the instance registered under the name derived from its type,
    /// when the context holds it, and returns it.
    ///
    /// Only the instance is removed; the provider that created it is kept, so
    /// resolving the instance again builds a new one. The name is the one
    /// [`default_singleton_name`] derives from the type, which is the name the
    /// attribute macros register an instance under.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn remove_singleton_with_default_name<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.remove_singleton_with_name::<T>(default_singleton_name::<T>())
    }

    /// Creates the instance registered under the given name, when it does not
    /// exist yet.
    ///
    /// The instance is built through its provider and stored in the context, so
    /// that a reference to it can be taken afterwards. This is what lets an item
    /// borrow an instance that was not inserted or resolved before.
    ///
    /// # Panics
    ///
    /// Panics when the context has no provider for the type and name.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    #[track_caller]
    fn just_create_singleton_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>)
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        if !self.try_just_create_singleton_with_name::<T>(key.name.clone()) {
            panic!("no instance could be created for: {key:?}");
        }
    }

    /// Creates the instance registered under the given name, when the context
    /// does not hold it yet, and reports whether it holds it afterwards.
    ///
    /// This is the variant of [`Self::just_create_singleton_with_name`] that
    /// does not panic: it lets an item that only optionally depends on another
    /// one create it first and decide afterwards.
    ///
    /// The result is `false` when the context has no provider for the type and
    /// name, and when the provider creates a new instance for every resolution
    /// (the [transient](crate::Scope::Transient) scope), because such an
    /// instance is not held by the context.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    fn try_just_create_singleton_with_name<T>(&mut self, name: impl Into<Cow<'static, str>>) -> bool
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        if self.contains_singleton_boxed(&key) {
            return true;
        }

        // Resolving a singleton stores it in the context. The owned copy the
        // resolution returns as well is dropped again, because the callers of
        // this method only want the instance to exist.
        let _ = self.resolve_boxed(&key);

        self.contains_singleton_boxed(&key)
    }

    /// Creates every instance of the given type the context does not hold yet,
    /// and reports for each of them whether it is held afterwards.
    ///
    /// The flag of an instance is `false` when its provider creates a new
    /// instance for every resolution, see
    /// [`Self::try_just_create_singleton_with_name`].
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instances.
    fn try_just_create_singletons_by_type<T>(&mut self) -> Vec<bool>
    where
        T: Send + Sync + 'static,
    {
        self.keys_of_type(TypeId::of::<T>())
            .into_iter()
            .map(|key| {
                if self.contains_singleton_boxed(&key) {
                    return true;
                }

                let _ = self.resolve_boxed(&key);
                self.contains_singleton_boxed(&key)
            })
            .collect()
    }

    /// Returns every instance of the given type the context holds.
    ///
    /// This is the borrowing counterpart of [`Self::resolve_by_type`], which
    /// creates the instances that are missing and hands them out as owned
    /// values. The instances are returned in the order of their keys.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instances.
    fn get_singletons_by_type<T>(&self) -> Vec<&T>
    where
        T: 'static,
    {
        self.keys_of_type(TypeId::of::<T>())
            .into_iter()
            .filter_map(|key| self.get_singleton_boxed(&key))
            .filter_map(|instance| instance.downcast_ref::<T>())
            .collect()
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

        self.resolve_singleton_option::<T>(&key)
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
        self.resolve_singleton_option::<T>(&key)
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

    /// Resolves the instance registered under the given key, when there is one.
    ///
    /// The instance is downcast to the type of the key. This is the shared,
    /// typed implementation of the `resolve*` methods; use
    /// [`Self::resolve_boxed`] to resolve an instance whose type is erased.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn resolve_singleton_option<T>(&mut self, key: &Key) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_boxed(key).map(|instance| {
            *instance
                .downcast::<T>()
                .expect("the context resolved an instance whose type does not match its key")
        })
    }
}

impl<C> ApplicationContextExt for C where C: ApplicationContext + ?Sized {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fmt;
    use std::{any::Any, fmt::Debug};

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

    impl Debug for FakeContext {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FakeContext")
                .field("instances", &"none")
                .finish()
        }
    }

    /// Type used to check the default name convention.
    #[derive(Clone)]
    struct MyService;

    #[test]
    fn inserts_and_reads_instances_by_name() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(7_u32, "answer");

        assert!(context.contains_singleton_with_name::<u32>("answer"));
        assert!(!context.contains_singleton_with_name::<u32>("missing"));
        assert_eq!(context.get_singleton_with_name::<u32>("answer"), &7);
    }

    #[test]
    fn resolves_owned_copies_of_instances() {
        let mut context = FakeContext::default();
        context.insert_singleton("value".to_owned());

        assert_eq!(context.resolve::<String>(), "value");
        assert_eq!(context.resolve_option::<String>(), Some("value".to_owned()));
    }

    #[test]
    fn resolves_unknown_instances_as_none() {
        let mut context = FakeContext::default();

        assert!(context.resolve_option::<u32>().is_none());
        assert!(context.get_singleton_option::<u32>().is_none());
    }

    #[test]
    fn changes_the_instance_a_mutable_reference_points_to() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(String::from("value"), "name");

        context
            .get_singleton_mut_with_name::<String>("name")
            .push('!');

        // The instance the context keeps is the one that was changed.
        assert_eq!(context.get_singleton_with_name::<String>("name"), "value!");
    }

    #[test]
    fn keeps_the_instance_that_exists_already() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(7_u32, "answer");

        context.just_create_singleton_with_name::<u32>("answer");

        assert_eq!(context.get_singleton_with_name::<u32>("answer"), &7);
    }

    #[test]
    #[should_panic(expected = "no instance could be created")]
    fn creating_an_instance_without_a_provider_panics() {
        let mut context = FakeContext::default();

        context.just_create_singleton_with_name::<u32>("answer");
    }

    #[test]
    fn uses_the_default_name_of_a_type() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_default_name(1_u64);
        context.insert_singleton_with_default_name(MyService);

        assert_eq!(
            context.get_singleton_option_with_default_name::<u64>(),
            Some(&1)
        );
        assert_eq!(context.resolve_with_default_name::<u64>(), 1);
        assert!(context.contains_singleton_with_default_name::<MyService>());
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

        assert_eq!(
            removed
                .and_then(|instance| instance.downcast::<u8>().ok())
                .map(|instance| *instance),
            Some(1)
        );
        assert!(!context.contains_singleton_with_name::<u8>("one"));
    }

    #[test]
    fn removes_the_instance_registered_under_a_name() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(7_u32, "answer");

        assert_eq!(context.remove_singleton_with_name::<u32>("answer"), Some(7));
        assert!(!context.contains_singleton_with_name::<u32>("answer"));

        // An instance the context does not hold is not returned.
        assert_eq!(context.remove_singleton_with_name::<u32>("answer"), None);
    }

    #[test]
    fn removes_the_instance_registered_under_the_empty_name() {
        let mut context = FakeContext::default();
        context.insert_singleton(7_u32);

        assert_eq!(context.remove_singleton::<u32>(), Some(7));
        assert!(!context.contains_singleton::<u32>());
    }

    #[test]
    fn removes_the_instance_registered_under_its_default_name() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_default_name(MyService);

        assert!(
            context
                .remove_singleton_with_default_name::<MyService>()
                .is_some()
        );
        assert!(!context.contains_singleton_with_default_name::<MyService>());
        assert!(
            context
                .remove_singleton_with_default_name::<MyService>()
                .is_none()
        );
    }

    #[test]
    fn reports_whether_an_instance_could_be_created() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(7_u32, "answer");

        // An instance the context holds is reported as created.
        assert!(context.try_just_create_singleton_with_name::<u32>("answer"));
        // An instance without a provider is not.
        assert!(!context.try_just_create_singleton_with_name::<u32>("missing"));
    }

    #[test]
    fn borrows_every_instance_of_a_type() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(1_i32, "one");
        context.insert_singleton_with_name(2_i32, "two");
        context.insert_singleton_with_name("other".to_owned(), "one");

        let mut values: Vec<i32> = context
            .get_singletons_by_type::<i32>()
            .into_iter()
            .copied()
            .collect();
        values.sort();

        assert_eq!(values, vec![1, 2]);
        // The instances of the other type are not returned.
        assert_eq!(context.get_singletons_by_type::<String>().len(), 1);
    }

    #[test]
    fn reports_every_instance_of_a_type() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(1_i32, "one");
        context.insert_singleton_with_name(2_i32, "two");

        // Both instances are held by the context, so both are reported.
        assert_eq!(
            context.try_just_create_singletons_by_type::<i32>(),
            vec![true, true]
        );
        // A type the context does not know at all has no instance to report.
        assert!(
            context
                .try_just_create_singletons_by_type::<u32>()
                .is_empty()
        );
    }
}
