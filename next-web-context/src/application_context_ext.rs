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
//!
//! The accessors are generic over a sized type, so an instance of a dynamic
//! trait is registered as `Arc<dyn Trait>` or as `Box<dyn Trait>`. The methods
//! whose name holds `dyn_singleton` register and look such an instance up under
//! the name derived from the trait object, because the default name of the
//! pointer type itself is not a name a singleton can hold. [`DynSingleton`] is
//! what makes code generic over the pointer type work with the trait object it
//! points at.

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::sync::Arc;

use next_web_singletons::factory::support::Key;
use next_web_singletons::util::SingletonUtil;

use crate::ApplicationContext;
use crate::application_context::InstanceClone;

/// A singleton a context stores behind a pointer to a trait object.
///
/// The type safe accessors of [`ApplicationContextExt`] are generic over a
/// sized type, because a context identifies an instance by the [`TypeId`] of
/// its type and hands it out by downcasting it. A trait object such as
/// `dyn UserDetails` is unsized and cannot be the type parameter of an
/// accessor, so a context stores an instance of a dynamic trait behind a
/// pointer: as `Arc<dyn UserDetails>` or as `Box<dyn UserDetails>`. The pointer,
/// and not the trait object, is the type a context keys the instance by, which
/// is why the dynamic accessors of [`ApplicationContextExt`] take the pointer
/// type as their type parameter.
///
/// `DynSingleton` is the seam between the two worlds: it hands out the unsized
/// target of the pointer, which is the trait object the callers work with. Code
/// that is generic over the pointer can accept an `Arc` and a `Box` alike and
/// still reach the trait object:
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_context::{ApplicationContext, ApplicationContextExt, DynSingleton};
///
/// trait UserDetails: Send + Sync {
///     fn is_enabled(&self) -> bool;
/// }
///
/// struct User;
///
/// impl UserDetails for User {
///     fn is_enabled(&self) -> bool {
///         true
///     }
/// }
///
/// /// Accepts `Arc<dyn UserDetails>` and `Box<dyn UserDetails>` alike.
/// fn is_enabled<P>(user_details: P) -> bool
/// where
///     P: DynSingleton,
///     P::Target: UserDetails,
/// {
///     user_details.target().is_enabled()
/// }
///
/// fn configure(context: &mut dyn ApplicationContext) {
///     // The instance is registered under the name of the trait object.
///     context.insert_dyn_singleton_with_default_name::<Arc<dyn UserDetails>>(Arc::new(User));
///
///     // The accessor peels the pointer, so the instance is a trait object.
///     let user_details: &dyn UserDetails =
///         context.get_dyn_singleton_with_default_name::<Arc<dyn UserDetails>>();
///     assert!(user_details.is_enabled());
///
///     let shared: Arc<dyn UserDetails> = Arc::new(User);
///     let boxed: Box<dyn UserDetails> = Box::new(User);
///
///     assert!(is_enabled(shared));
///     assert!(is_enabled(boxed));
/// }
/// ```
///
/// # Provided implementations
///
/// * `Arc<T>` - borrows the instance that is already shared,
/// * `Box<T>` - owns the instance, which [`Self::into_target_arc`] moves into an
///   `Arc` without copying it.
///
/// Those are the two pointers the framework registers an instance of a dynamic
/// trait in. Another pointer type can hand out its target the same way by
/// implementing the trait as well.
pub trait DynSingleton {
    /// The unsized target of the pointer, for example `dyn UserDetails`.
    type Target: ?Sized + 'static;

    /// Borrows the target of the pointer.
    fn target(&self) -> &Self::Target;

    /// Moves the target into an [`Arc`], which is the pointer the framework
    /// hands the instances of a dynamic trait out with.
    ///
    /// An instance that lives in an `Arc` already is shared as it is, so the
    /// `Arc` is returned unchanged. Moving a `Box` into an `Arc` keeps the
    /// allocation of the box instead of copying the instance into a new one.
    fn into_target_arc(self) -> Arc<Self::Target>
    where
        Self: Sized;

    /// Returns the name a provider registers a singleton of this pointer type
    /// under by default.
    ///
    /// The name is the one [`SingletonUtil::default_singleton_name`] derives
    /// from the *target* of the pointer, so `Arc<dyn UserDetails>` and
    /// `Box<dyn UserDetails>` both provide `userDetails`. The name cannot be
    /// derived from the pointer type, because its type name
    /// (`alloc::sync::Arc<dyn ...::UserDetails>`) holds characters a name cannot
    /// hold. A provider of a dynamic trait registers its instance under this
    /// name, which is the name
    /// [`ApplicationContextExt::get_dyn_singleton_with_default_name`] looks up.
    fn default_singleton_name() -> String {
        SingletonUtil::default_singleton_name::<Self::Target>()
    }
}

impl<T> DynSingleton for Arc<T>
where
    T: ?Sized + 'static,
{
    type Target = T;

    fn target(&self) -> &T {
        self
    }

    fn into_target_arc(self) -> Arc<T> {
        self
    }
}

impl<T> DynSingleton for Box<T>
where
    T: ?Sized + 'static,
{
    type Target = T;

    fn target(&self) -> &T {
        self
    }

    fn into_target_arc(self) -> Arc<T> {
        Arc::from(self)
    }
}

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
///   from its type by [`SingletonUtil::default_singleton_name`].
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
        self.insert_singleton_with_name(instance, SingletonUtil::default_singleton_name::<T>());
    }

    /// Inserts `instance` under the name derived from the trait object.
    ///
    /// This is the dynamic counterpart of
    /// [`Self::insert_singleton_with_default_name`], which cannot name an
    /// instance of a dynamic trait: the default name of a pointer type such as
    /// `Arc<dyn UserDetails>` holds the characters of the pointer
    /// (`alloc::sync::Arc<dyn ...::UserDetails>`) and is not a name a singleton
    /// can be registered under. The name is therefore the one
    /// [`DynSingleton::default_singleton_name`] derives from the trait object,
    /// which is the name a provider of a dynamic trait registers its instance
    /// under.
    ///
    /// An instance of a dynamic trait is shared, so the context stores it as an
    /// `Arc<dyn UserDetails>`: an `Arc` is stored as it is, and a `Box` is moved
    /// into an `Arc` first, which keeps the allocation of the box. The instance
    /// is registered with a clone function, so that the context hands out owned
    /// copies of it as well, and the pointer to look it up with afterwards is
    /// the `Arc`. Registering the instance under another name, and registering
    /// it in another pointer type than the `Arc`, is what the ordinary
    /// [`Self::insert_singleton_with_name`] is for.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance to insert.
    ///
    /// # Type Parameters
    ///
    /// * `P` - The pointer type, for example `Arc<dyn UserDetails>`.
    fn insert_dyn_singleton_with_default_name<P>(&mut self, instance: P)
    where
        P: DynSingleton,
        P::Target: Send + Sync + 'static,
    {
        self.insert_singleton_with_name::<Arc<P::Target>>(
            instance.into_target_arc(),
            P::default_singleton_name(),
        );
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
        self.contains_singleton_with_name::<T>(SingletonUtil::default_singleton_name::<T>())
    }

    /// Returns whether an instance of a dynamic trait is registered under the
    /// name derived from the trait object.
    ///
    /// This is the dynamic counterpart of
    /// [`Self::contains_singleton_with_default_name`], which cannot name an
    /// instance of a dynamic trait, see
    /// [`Self::insert_dyn_singleton_with_default_name`]. The name is the one
    /// [`DynSingleton::default_singleton_name`] derives from the trait object,
    /// so this reports the instance a provider of a dynamic trait registered by
    /// default.
    ///
    /// The pointer the instance is stored in is the type parameter, because it
    /// is part of the key: an instance registered as an `Arc<dyn UserDetails>`
    /// and an instance registered as a `Box<dyn UserDetails>` are two different
    /// singletons.
    ///
    /// # Type Parameters
    ///
    /// * `P` - The pointer type, for example `Arc<dyn UserDetails>`.
    fn contains_dyn_singleton_with_default_name<P>(&self) -> bool
    where
        P: DynSingleton + 'static,
    {
        self.contains_singleton_with_name::<P>(P::default_singleton_name())
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
        self.get_singleton_option_with_name::<T>(SingletonUtil::default_singleton_name::<T>())
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

    /// Returns the trait object of the singleton registered under the name
    /// derived from the trait object.
    ///
    /// This is the dynamic counterpart of [`Self::get_singleton_with_name`]: the
    /// type parameter is the pointer the instance is stored in, and the
    /// returned reference points at the unsized target of that pointer, which
    /// is the trait object the callers work with. The name is the one
    /// [`DynSingleton::default_singleton_name`] derives from the trait object,
    /// which is the name a provider of a dynamic trait registers its instance
    /// under, see [`Self::insert_dyn_singleton_with_default_name`]. A pointer
    /// type has no default name a singleton could be registered under, so the
    /// ordinary accessors cannot look the instance up by a default name.
    ///
    /// The pointer is part of the key, so an instance registered as an
    /// `Arc<dyn UserDetails>` and an instance registered as a
    /// `Box<dyn UserDetails>` are two different singletons.
    ///
    /// # Panics
    ///
    /// Panics when no instance is registered for the pointer type.
    ///
    /// # Type Parameters
    ///
    /// * `P` - The pointer type, for example `Arc<dyn UserDetails>`.
    #[track_caller]
    fn get_dyn_singleton_with_default_name<P>(&self) -> &P::Target
    where
        P: DynSingleton + 'static,
    {
        self.get_singleton_with_name::<P>(P::default_singleton_name())
            .target()
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
    /// [`SingletonUtil::default_singleton_name`] derives from the type, which is
    /// the name the attribute macros register an instance under.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    fn remove_singleton_with_default_name<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.remove_singleton_with_name::<T>(SingletonUtil::default_singleton_name::<T>())
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
        self.resolve_with_name::<T>(SingletonUtil::default_singleton_name::<T>())
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
        self.resolve_option_with_name::<T>(SingletonUtil::default_singleton_name::<T>())
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
    /// [`ApplicationContext::resolve_boxed`] to resolve an instance whose type
    /// is erased.
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
    use super::*;
    use crate::test_support::FakeContext;

    /// Type used to check the default name convention.
    #[derive(Clone)]
    struct MyService;

    /// Trait that is used the way the framework uses a dynamic trait: the
    /// instance of such a trait is registered as `Arc<dyn Greeter>` or as
    /// `Box<dyn Greeter>`, because the trait object itself cannot be named as
    /// the type of a stored instance.
    trait Greeter: Send + Sync {
        fn greet(&self) -> String;
    }

    struct HelloGreeter;

    impl Greeter for HelloGreeter {
        fn greet(&self) -> String {
            "hello".to_owned()
        }
    }

    /// Function that is generic over the pointer type the trait object is
    /// stored in, which is what [`DynSingleton`] is for.
    fn greet_generically<P>(greeter: P) -> String
    where
        P: DynSingleton,
        P::Target: Greeter,
    {
        let greeting = greeter.target().greet();
        let shared: Arc<P::Target> = greeter.into_target_arc();

        // Both pointers lead to the same instance.
        assert_eq!(shared.greet(), greeting);

        greeting
    }

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
        assert_eq!(
            SingletonUtil::default_singleton_name::<MyService>(),
            "myService"
        );
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

    #[test]
    fn peels_the_trait_object_generically_over_the_pointer() {
        let shared: Arc<dyn Greeter> = Arc::new(HelloGreeter);
        let boxed: Box<dyn Greeter> = Box::new(HelloGreeter);

        assert_eq!(greet_generically(shared), "hello");
        assert_eq!(greet_generically(boxed), "hello");
    }

    #[test]
    fn derives_the_default_name_of_a_dynamic_trait_from_the_trait_object() {
        // The name of the trait object is the one that can be a singleton name,
        // so both pointers provide the same default name.
        assert_eq!(Arc::<dyn Greeter>::default_singleton_name(), "greeter");
        assert_eq!(Box::<dyn Greeter>::default_singleton_name(), "greeter");

        let mut context = FakeContext::default();
        let greeter: Box<dyn Greeter> = Box::new(HelloGreeter);
        context.insert_singleton_boxed(
            Key::new::<Box<dyn Greeter>>(Box::<dyn Greeter>::default_singleton_name().into()),
            Box::new(greeter),
            None,
        );

        assert_eq!(
            context
                .get_dyn_singleton_with_default_name::<Box<dyn Greeter>>()
                .greet(),
            "hello"
        );
        // The pointer is part of the key, so the other one holds no instance.
        assert!(!context.contains_dyn_singleton_with_default_name::<Arc<dyn Greeter>>());
    }

    #[test]
    #[should_panic(expected = "no instance is registered")]
    fn peeling_an_unknown_dynamic_singleton_panics() {
        let context = FakeContext::default();

        let _ = context.get_dyn_singleton_with_default_name::<Arc<dyn Greeter>>();
    }

    #[test]
    fn registers_and_reads_a_dynamic_singleton_under_the_default_name() {
        let mut context = FakeContext::default();
        context.insert_dyn_singleton_with_default_name::<Box<dyn Greeter>>(Box::new(HelloGreeter));

        // The instance is stored as a shared trait object under the name of the
        // trait object, so the `Arc` is the pointer it is read with.
        assert!(context.contains_dyn_singleton_with_default_name::<Arc<dyn Greeter>>());
        assert_eq!(
            context
                .get_dyn_singleton_with_default_name::<Arc<dyn Greeter>>()
                .greet(),
            "hello"
        );

        // The instance is registered with a clone function, so the context hands
        // out owned copies of it as well.
        let name = Arc::<dyn Greeter>::default_singleton_name();
        assert_eq!(
            context.resolve_with_name::<Arc<dyn Greeter>>(name).greet(),
            "hello"
        );

        // The default name of the pointer type is not the name the instance is
        // registered under, which is why the dynamic accessors derive the name
        // from the trait object.
        assert!(!context.contains_singleton_with_default_name::<Arc<dyn Greeter>>());
    }

    #[test]
    fn registers_a_dynamic_singleton_under_a_name_through_the_ordinary_method() {
        let mut context = FakeContext::default();

        // Another name than the default name of the trait object, and another
        // pointer than the `Arc` a dynamic insert stores, is what the ordinary
        // accessors are for.
        let boxed: Box<dyn Greeter> = Box::new(HelloGreeter);
        context.insert_singleton_with_name::<Arc<dyn Greeter>>(Arc::from(boxed), "greeter");

        assert!(context.contains_singleton_with_name::<Arc<dyn Greeter>>("greeter"));
        // The pointer is part of the key, so the box is not registered.
        assert!(!context.contains_singleton_with_name::<Box<dyn Greeter>>("greeter"));

        // The caller peels the pointer itself, which is all a dynamic accessor
        // of a named instance would do as well.
        let greeter: &dyn Greeter =
            &**context.get_singleton_with_name::<Arc<dyn Greeter>>("greeter");
        assert_eq!(greeter.greet(), "hello");
    }
}
