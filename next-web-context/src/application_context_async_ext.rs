//! Type safe helpers built on top of [`ApplicationContext`] that await the
//! constructors of the providers they resolve.
//!
//! The accessors of [`ApplicationContextExt`](crate::ApplicationContextExt)
//! resolve the providers of a context synchronously, so a provider whose
//! constructor is an `async fn` cannot be resolved by them. The methods of
//! [`ApplicationContextAsyncExt`] are the counterparts that await such a
//! constructor, and they are implemented for every context, including
//! `dyn ApplicationContext`:
//!
//! ```
//! use next_web_context::{ApplicationContext, ApplicationContextAsyncExt};
//!
//! async fn configure(context: &mut dyn ApplicationContext) {
//!     // `ApplicationContextAsyncExt` has to be in scope for this call.
//!     let _ = context.resolve_option_async::<u32>().await;
//! }
//! ```

use std::borrow::Cow;

use async_trait::async_trait;
use next_web_singletons::factory::support::Key;
use next_web_singletons::util::SingletonUtil;

use crate::ApplicationContext;

/// Type safe, asynchronous access to the singletons of an
/// [`ApplicationContext`].
///
/// The trait is blanket implemented, so it is enough to import it to call its
/// methods on any context:
///
/// ```
/// use next_web_context::{ApplicationContext, ApplicationContextAsyncExt};
///
/// async fn configure(context: &mut dyn ApplicationContext) {
///     let _ = context.resolve_with_name_async::<u32>("answer").await;
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
/// The methods are the asynchronous counterparts of the methods of
/// [`ApplicationContextExt`](crate::ApplicationContextExt), and the instances
/// they hand out are the same ones.
#[async_trait]
pub trait ApplicationContextAsyncExt: ApplicationContext {
    /// Resolves the instance registered under the empty name `""`.
    ///
    /// The instance is created by awaiting the constructor of its provider,
    /// which is what lets a provider that is created asynchronously be
    /// resolved.
    ///
    /// # Panics
    ///
    /// Panics when the instance cannot be resolved.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    #[track_caller]
    async fn resolve_async<T>(&mut self) -> T
    where
        T: Send + Sync + 'static,
    {
        self.resolve_with_name_async::<T>("").await
    }

    /// Resolves the instance registered under the empty name `""`, when there
    /// is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    async fn resolve_option_async<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_option_with_name_async::<T>("").await
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
    async fn resolve_with_name_async<T>(&mut self, name: impl Into<Cow<'static, str>> + Send) -> T
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        self.resolve_singleton_option_async::<T>(&key)
            .await
            .unwrap_or_else(|| panic!("no instance could be resolved for: {key:?}"))
    }

    /// Resolves the instance registered under the given name, when there is
    /// one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    async fn resolve_option_with_name_async<T>(
        &mut self,
        name: impl Into<Cow<'static, str>> + Send,
    ) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        self.resolve_singleton_option_async::<T>(&key).await
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
    async fn resolve_with_default_name_async<T>(&mut self) -> T
    where
        T: Send + Sync + 'static,
    {
        self.resolve_with_name_async::<T>(SingletonUtil::default_singleton_name::<T>())
            .await
    }

    /// Resolves the instance registered under the name derived from its type,
    /// when there is one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    async fn resolve_option_with_default_name_async<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_option_with_name_async::<T>(SingletonUtil::default_singleton_name::<T>())
            .await
    }

    /// Resolves every instance of the given type.
    ///
    /// The instances are returned in the order of their keys.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instances to resolve.
    async fn resolve_by_type_async<T>(&mut self) -> Vec<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_all_of_type_async(std::any::TypeId::of::<T>())
            .await
            .into_iter()
            .filter_map(|instance| instance.downcast::<T>().ok().map(|instance| *instance))
            .collect()
    }

    /// Resolves every instance of the given type, with the names they are
    /// registered under.
    ///
    /// The named counterpart of [`Self::resolve_by_type_async`], and the async
    /// counterpart of
    /// [`ApplicationContextExt::resolve_by_type_with_names`](crate::ApplicationContextExt::resolve_by_type_with_names).
    ///
    /// The names are taken from the keys of the type and paired with the instances
    /// [`Self::resolve_by_type_async`] returns, which its own documentation
    /// promises to hand back in the order of their keys. Both walk the same set of
    /// keys for the same `TypeId`, so the two orders are the same order.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instances to resolve.
    async fn resolve_by_type_with_names_async<T>(&mut self) -> Vec<(String, T)>
    where
        T: Send + Sync + 'static,
    {
        let names = self
            .keys_of_type(std::any::TypeId::of::<T>())
            .into_iter()
            .map(|key| key.name.to_string());

        names.zip(self.resolve_by_type_async::<T>().await).collect()
    }

    /// Creates the instance registered under the given name, when it does not
    /// exist yet.
    ///
    /// The instance is created by awaiting the constructor of its provider.
    ///
    /// # Panics
    ///
    /// Panics when the instance cannot be created and held by the context.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    #[track_caller]
    async fn just_create_singleton_with_name_async<T>(
        &mut self,
        name: impl Into<Cow<'static, str>> + Send,
    ) where
        T: Send + Sync + 'static,
    {
        let key = Key::new::<T>(name.into());

        if !self
            .try_just_create_singleton_with_name_async::<T>(key.name.clone())
            .await
        {
            panic!("no instance could be created for: {key:?}");
        }
    }

    /// Creates the instance registered under the given name, when the context
    /// does not hold it yet, and reports whether it holds it afterwards.
    ///
    /// The flag is `false` when the context has no provider for the type and
    /// name, and when the provider creates a new instance for every resolution
    /// (the [transient](crate::Scope::Transient) scope), because such an
    /// instance is not held by the context.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    /// * `N` - The name type.
    async fn try_just_create_singleton_with_name_async<T>(
        &mut self,
        name: impl Into<Cow<'static, str>> + Send,
    ) -> bool
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
        let _ = self.resolve_boxed_async(&key).await;

        self.contains_singleton_boxed(&key)
    }

    /// Creates every instance of the given type the context does not hold yet,
    /// and reports for each of them whether it is held afterwards.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instances.
    async fn try_just_create_singletons_by_type_async<T>(&mut self) -> Vec<bool>
    where
        T: Send + Sync + 'static,
    {
        let mut created = Vec::new();

        for key in self.keys_of_type(std::any::TypeId::of::<T>()) {
            if self.contains_singleton_boxed(&key) {
                created.push(true);

                continue;
            }

            let _ = self.resolve_boxed_async(&key).await;
            created.push(self.contains_singleton_boxed(&key));
        }

        created
    }

    /// Resolves the instance registered under the given key, when there is one.
    ///
    /// The instance is downcast to the type of the key. This is the shared,
    /// typed implementation of the `resolve*` methods.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    async fn resolve_singleton_option_async<T>(&mut self, key: &Key) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.resolve_boxed_async(key).await.map(|instance| {
            *instance
                .downcast::<T>()
                .expect("the context resolved an instance whose type does not match its key")
        })
    }
}

#[async_trait]
impl<C> ApplicationContextAsyncExt for C where C: ApplicationContext + ?Sized {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeContext;
    use crate::{
        ApplicationContextExt, DynProvider, FutureExt, Provider, SingletonAsyncProvider,
        singleton_async,
    };

    /// Returns a provider whose constructor is an `async fn`.
    fn async_provider() -> DynProvider {
        let provider: SingletonAsyncProvider<String> =
            singleton_async(|_| async { String::from("awaited") }.boxed());
        let provider: Provider<String> = provider.name("greeting").into();

        provider.into()
    }

    #[tokio::test]
    async fn awaits_the_constructor_of_a_provider() {
        let provider = async_provider();
        let mut context = FakeContext::default();

        let instance = provider.build_boxed_async(&mut context).await;

        assert_eq!(
            *instance
                .downcast::<String>()
                .expect("the provider creates a string"),
            "awaited"
        );
    }

    #[test]
    #[should_panic(expected = "is created asynchronously")]
    fn does_not_create_a_provider_asynchronously_resolved_by_a_synchronous_context() {
        let provider = async_provider();
        let mut context = FakeContext::default();

        let _ = provider.build_boxed(&mut context);
    }

    #[tokio::test]
    async fn resolves_the_instances_the_context_holds() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(String::from("hello"), "greeting");

        assert_eq!(
            context.resolve_with_name_async::<String>("greeting").await,
            "hello"
        );
        assert_eq!(
            context
                .resolve_option_with_name_async::<String>("missing")
                .await,
            None
        );
    }

    #[tokio::test]
    async fn resolves_every_instance_of_a_type() {
        let mut context = FakeContext::default();
        context.insert_singleton_with_name(1_i32, "one");
        context.insert_singleton_with_name(2_i32, "two");
        context.insert_singleton_with_name(String::from("other"), "one");

        let mut values = context.resolve_by_type_async::<i32>().await;
        values.sort();

        assert_eq!(values, vec![1, 2]);
    }

    #[tokio::test]
    async fn reports_an_instance_the_context_cannot_create() {
        let mut context = FakeContext::default();

        assert!(
            !context
                .try_just_create_singleton_with_name_async::<u32>("missing")
                .await
        );
        assert!(
            context
                .try_just_create_singletons_by_type_async::<u32>()
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    #[should_panic(expected = "no instance could be created")]
    async fn creates_an_instance_the_context_cannot_create() {
        let mut context = FakeContext::default();

        context
            .just_create_singleton_with_name_async::<u32>("missing")
            .await;
    }
}
