use std::any::{self, Any};
use std::cmp::Ordering;
use std::collections::hash_map::Keys;
use std::{
    any::TypeId, borrow::Cow, collections::HashMap, future::Future, hash::Hasher, pin::Pin,
    sync::Arc,
};


use std::hash::Hash;

use crate::autoregister::auto_register::AutoRegisterModule;

/// A context is a container for all the providers and instances.
///
/// It is the main entry point for the dependency injection.
/// It is also used to create new instances.
///
/// When creating a `ApplicationContext`, you can use options to change the
/// default creation behavior, see [`ApplicationContextOptions`] for details.
///
/// # Example
///
/// Creating context with customized modules:
/// ```rust
/// use rudi::{components, modules, ApplicationContext, DynProvider, Module, Transient};
///
/// #[Transient]
/// struct A;
///
/// struct Module1;
///
/// impl Module for Module1 {
///     fn providers() -> Vec<DynProvider> {
///         components![A]
///     }
/// }
///
/// #[derive(Debug)]
/// #[Transient]
/// struct B;
///
/// struct Module2;
///
/// impl Module for Module2 {
///     fn providers() -> Vec<DynProvider> {
///         components![B]
///     }
/// }
///
/// # fn main() {
/// let mut cx = ApplicationContext::create(modules![Module1, Module2]);
///
/// let b = cx.resolve::<B>();
///
/// assert!(cx.resolve_option::<A>().is_some());
/// assert_eq!(format!("{:?}", b), "B");
/// # }
/// ```
///
/// With the `auto-register` feature enabled (which is enabled by default),
/// it is also possible to create contexts in a simpler way:
/// ```rust
/// use rudi::{ApplicationContext, Transient};
///
/// #[Transient]
/// struct A;
///
/// #[derive(Debug)]
/// #[Transient]
/// struct B;
///
/// # fn main() {
/// let mut cx = ApplicationContext::auto_register();
/// // This is a simplified version of the following
/// // let mut cx = ApplicationContext::create(modules![AutoRegisterModule]);
///
/// let b = cx.resolve::<B>();
///
/// assert!(cx.resolve_option::<A>().is_some());
/// assert_eq!(format!("{:?}", b), "B");
/// # }
/// ```
///
/// If the following conditions are met:
/// 1. in context, there exists a provider whose constructor is async.
/// 2. the `eager_create` method of the provider is set to true, e.g., [`SingletonProvider::eager_create`](crate::SingletonProvider::eager_create).
/// 3. the `eager_create` method of the module to which the provide belongs is set to true, i.e., [`Module::eager_create`](crate::Module::eager_create).
/// 4. the `eager_create` field of the context, is set to true, i.e., [`ApplicationContextOptions::eager_create`].
///
/// Then when creating the context, you must use the async creation methods, [`ApplicationContext::create_async`] or [`ApplicationContext::auto_register_async`]:
///
/// ```rust
/// use rudi::{ApplicationContext, Singleton, Transient};
///
/// #[Singleton]
/// async fn Foo() -> i32 {
///     1
/// }
///
/// #[derive(Debug)]
/// #[Transient(async)]
/// struct A(i32);
///
/// #[tokio::main]
/// async fn main() {
///     let mut cx = ApplicationContext::options()
///         .eager_create(true)
///         .auto_register_async()
///         .await;
///
///     assert!(cx.resolve_option_async::<A>().await.is_some());
/// }
/// ```
#[derive(Clone)]
pub struct ApplicationContext {
    allow_override: bool,
    allow_only_single_eager_create: bool,

    eager_create: bool,

    single_registry: SingleRegistry,
    provider_registry: ProviderRegistry,

    loaded_modules: Vec<Type>,
    conditional_providers: Vec<(bool, DynProvider)>,
    eager_create_functions: Vec<(Definition, EagerCreateFunction)>,

    dependency_chain: DependencyChain,
}

impl Default for ApplicationContext {
    fn default() -> Self {
        Self {
            allow_override: true,
            allow_only_single_eager_create: true,
            eager_create: Default::default(),
            single_registry: Default::default(),
            provider_registry: Default::default(),
            loaded_modules: Default::default(),
            conditional_providers: Default::default(),
            eager_create_functions: Default::default(),
            dependency_chain: Default::default(),
        }
    }
}

impl ApplicationContext {
    /// Creates a new context with the given modules.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{components, modules, ApplicationContext, DynProvider, Module, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers() -> Vec<DynProvider> {
    ///         components![A]
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::create(modules![MyModule]);
    /// assert!(cx.resolve_option::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn create(modules: Vec<ResolveModule>) -> ApplicationContext {
        ApplicationContextOptions::default().create(modules)
    }

    /// Creates a new context with the [`AutoRegisterModule`].
    ///
    /// Same as `ApplicationContext::create(modules![AutoRegisterModule])`.
    ///
    /// See [`ApplicationContext::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// [`AutoRegisterModule`]: crate::AutoRegisterModule
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[track_caller]
    pub fn auto_register() -> ApplicationContext {
        ApplicationContextOptions::default().auto_register()
    }

    /// Async version of [`ApplicationContext::create`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ApplicationContext::create`].
    ///
    /// See [`ApplicationContext::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn create_async(modules: Vec<ResolveModule>) -> ApplicationContext {
        ApplicationContextOptions::default().create_async(modules).await
    }

    /// Async version of [`ApplicationContext::auto_register`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ApplicationContext::auto_register`].
    ///
    /// See [`ApplicationContext::auto_register`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    pub async fn auto_register_async() -> ApplicationContext {
        ApplicationContextOptions::default().auto_register_async().await
    }

    /// Returns a new ApplicationContextOptions object.
    ///
    /// This function return a new ApplicationContextOptions object that you can use to create a context with specific options
    /// if `create()` or `auto_register()` are not appropriate.
    ///
    /// It is equivalent to `ApplicationContextOptions::default()`, but allows you to write more readable code.
    /// Instead of `ApplicationContextOptions::default().eager_create(true).auto_register()`,
    /// you can write `ApplicationContext::options().eager_create(true).auto_register()`.
    /// This also avoids the need to import `ApplicationContextOptions`.
    ///
    /// See the [`ApplicationContextOptions`] for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::options().eager_create(true).auto_register();
    ///
    /// assert!(cx.contains_single::<A>());
    /// # }
    /// ```
    pub fn options() -> ApplicationContextOptions {
        ApplicationContextOptions::default()
    }

    /// Returns whether the context should allow overriding existing providers.
    pub fn allow_override(&self) -> bool {
        self.allow_override
    }

    /// Returns whether the context should only eagerly create [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances.
    pub fn allow_only_single_eager_create(&self) -> bool {
        self.allow_only_single_eager_create
    }

    /// Returns whether the context should eagerly create instances.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns a reference to the single registry.
    pub fn single_registry(&self) -> &HashMap<Key, DynSingle> {
        self.single_registry.inner()
    }

    /// Returns a reference to the provider registry.
    pub fn provider_registry(&self) -> &HashMap<Key, DynProvider> {
        self.provider_registry.inner()
    }

    /// Returns a reference to the loaded modules.
    pub fn loaded_modules(&self) -> &Vec<Type> {
        &self.loaded_modules
    }

    /// Returns a reference to the conditional providers.
    pub fn conditional_providers(&self) -> &Vec<(bool, DynProvider)> {
        &self.conditional_providers
    }

    /// Returns a reference to the eager create functions.
    pub fn eager_create_functions(&self) -> &Vec<(Definition, EagerCreateFunction)> {
        &self.eager_create_functions
    }

    /// Returns a reference to the dependency chain.
    pub fn dependency_chain(&self) -> &Vec<Key> {
        &self.dependency_chain.stack
    }

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `ApplicationContext` and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::ApplicationContext;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    /// cx.insert_singleton(42);
    /// assert_eq!(cx.get_single::<i32>(), &42);
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_singleton<T>(&mut self, instance: T)
    where
        T: 'static + Clone + Send + Sync,
    {
        self.insert_singleton_with_name(instance, "");
    }

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with name.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `ApplicationContext` and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::ApplicationContext;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    ///
    /// cx.insert_singleton_with_name(1, "one");
    /// cx.insert_singleton_with_name(2, "two");
    ///
    /// assert_eq!(cx.get_single_with_name::<i32>("one"), &1);
    /// assert_eq!(cx.get_single_with_name::<i32>("two"), &2);
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_singleton_with_name<T, N>(&mut self, instance: T, name: N)
    where
        T: 'static + Clone + Send + Sync,
        N: Into<Cow<'static, str>>,
    {
        let name = name.into();
        let provider: DynProvider =
            Provider::<T>::never_construct(name.clone(), Scope::Singleton).into();
        let single = Single::new(instance, Some(Clone::clone)).into();

        let key = provider.key().clone();
        self.provider_registry.insert(provider, self.allow_override);
        self.single_registry.insert(key.clone(), single);

        println!(
            "Singleton registry insert registered successfully!\nkey: {:?}\n",
            key
        );
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `ApplicationContext` and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::ApplicationContext;
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    /// cx.insert_single_owner(NotClone(42));
    /// assert_eq!(cx.get_single::<NotClone>(), &NotClone(42));
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_single_owner<T>(&mut self, instance: T)
    where
        T: 'static + Send + Sync,
    {
        self.insert_single_owner_with_name(instance, "");
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with name.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `ApplicationContext` and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::ApplicationContext;
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    ///
    /// cx.insert_single_owner_with_name(NotClone(1), "one");
    /// cx.insert_single_owner_with_name(NotClone(2), "two");
    ///
    /// assert_eq!(cx.get_single_with_name::<NotClone>("one"), &NotClone(1));
    /// assert_eq!(cx.get_single_with_name::<NotClone>("two"), &NotClone(2));
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_single_owner_with_name<T, N>(&mut self, instance: T, name: N)
    where
        T: 'static + Send + Sync,
        N: Into<Cow<'static, str>>,
    {
        let provider: DynProvider =
            Provider::<T>::never_construct(name.into(), Scope::SingleOwner).into();
        let single = Single::new(instance, None).into();

        let key = provider.key().clone();
        self.provider_registry.insert(provider, self.allow_override);
        self.single_registry.insert(key.clone(), single);

        println!(
            "SingleOwner registry insert registered successfully!\nkey: {:?}\n",
            key
        );
    }

    /// Load the given modules.
    ///
    /// This method first flattens all the given modules together with their submodules
    /// into a collection of modules without submodules, then takes out the providers of
    /// each module in this collection, flattens all the providers together with their
    /// bound providers into a collection of providers without bound providers, and finally
    /// deposits the providers one by one into context.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    /// assert!(cx.get_provider::<A>().is_none());
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    /// assert!(cx.get_provider::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn load_modules(&mut self, modules: Vec<ResolveModule>) {
        if modules.is_empty() {
            return;
        }

        let modules = flatten(modules, ResolveModule::submodules);

        modules.into_iter().for_each(|module| {
            self.loaded_modules.push(module.ty());
            self.load_providers(module.eager_create(), module.providers());
        });
    }

    /// Unload the given modules.
    ///
    /// This method will convert the given module into a collection of providers like
    /// the [`ApplicationContext::load_modules`] method, and then remove all providers in the context
    /// that are equal to the providers in the collection and their possible instances.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    /// assert!(cx.get_provider::<A>().is_none());
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    /// assert!(cx.get_provider::<A>().is_some());
    ///
    /// cx.unload_modules(modules![AutoRegisterModule]);
    /// assert!(cx.get_provider::<A>().is_none());
    /// # }
    /// ```
    pub fn unload_modules(&mut self, modules: Vec<ResolveModule>) {
        if modules.is_empty() {
            return;
        }

        let modules = flatten(modules, ResolveModule::submodules);

        modules.into_iter().for_each(|module| {
            self.loaded_modules.retain(|ty| ty != &module.ty());
            self.unload_providers(module.providers());
        });
    }

    /// Flush the context.
    ///
    /// This method has two purposes:
    ///
    /// 1. Evaluate the condition of providers whose [`condition`](crate::Provider::condition) is `Some`.
    ///
    ///    If the evaluation result is `true`, the provider will be loaded into the context,
    ///    otherwise it will be removed from the context.
    ///
    /// 2. Construct instances that will be eagerly created.
    ///
    ///    Whether an instance need to be created eagerly depends on
    ///    the [`eager_create`](crate::Provider::eager_create) field of the Provider that defines it,
    ///    the [`eager_create`](crate::ResolveModule::eager_create) field of the Module to which this Provider belongs,
    ///    and the [`eager_create`](crate::ApplicationContext::eager_create) field of the ApplicationContext to which this Module belongs.
    ///    As long as one of these is true, the instance need to be created eagerly.
    ///
    ///    Whether an instance is allowed to be created eagerly depends on
    ///    the [`scope`](crate::Definition::scope) field in the [`definition`](crate::Provider::definition) field of the Provider that defines it,
    ///    and the [`allow_only_single_eager_create`](crate::ApplicationContext::allow_only_single_eager_create) field of the ApplicationContext to which this Provider belongs.
    ///    If `allow_only_single_eager_create` is false, or `allow_only_single_eager_create` is true and `scope` is [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    ///    the instance is allowed to be created eagerly.
    ///
    ///    When an instance need to be created eagerly and is allowed to be created eagerly, it will be created eagerly.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, ApplicationContext, Singleton, Transient};
    ///
    /// #[Transient(condition = |_| true)]
    /// struct A;
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create)]
    /// struct B;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::default();
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    ///
    /// assert!(!cx.contains_provider::<A>());
    /// assert!(!cx.contains_single::<B>());
    ///
    /// cx.flush();
    ///
    /// // evaluate condition
    /// assert!(cx.contains_provider::<A>());
    /// // construct instance
    /// assert!(cx.contains_single::<B>());
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// This method needs to be called after the [`ApplicationContext::load_modules`] method,
    /// but why not put the logic of this method in the `load_modules` method? Please see the example below:
    ///
    /// ```rust
    /// use rudi::{components, modules, ApplicationContext, DynProvider, Module, Transient};
    ///
    /// fn a_condition(cx: &ApplicationContext) -> bool {
    ///     cx.contains_provider::<B>()
    /// }
    ///
    /// #[Transient(condition = a_condition)]
    /// struct A;
    ///
    /// #[Transient]
    /// struct B;
    ///
    /// struct AModule;
    ///
    /// impl Module for AModule {
    ///     fn providers() -> Vec<DynProvider> {
    ///         components![A]
    ///     }
    /// }
    ///
    /// struct BModule;
    ///
    /// impl Module for BModule {
    ///     fn providers() -> Vec<DynProvider> {
    ///         components![B]
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let mut cx = ApplicationContext::default();
    ///
    ///     // Method 1, call `load_modules` and then call `flush` immediately
    ///     cx.load_modules(modules![AModule]);
    ///     cx.flush();
    ///     cx.load_modules(modules![BModule]);
    ///     cx.flush();
    ///
    ///     // The evaluation result of `A`'s `condition` is `false`, so `A` will not be created
    ///     assert!(!cx.contains_provider::<A>());
    ///
    ///     let mut cx = ApplicationContext::default();
    ///
    ///     // Method 2, call all `load_modules` first, then call `flush`
    ///     cx.load_modules(modules![AModule]);
    ///     cx.load_modules(modules![BModule]);
    ///     cx.flush();
    ///
    ///     // The evaluation result of `A`'s `condition` is `true`, so `A` will be created
    ///     assert!(cx.contains_provider::<A>());
    /// }
    /// ```
    #[track_caller]
    pub fn flush(&mut self) {
        self.create_eager_instances();

        self.evaluate_providers();
        self.create_eager_instances();
    }

    /// Async version of [`ApplicationContext::flush`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ApplicationContext::flush`].
    ///
    /// See [`ApplicationContext::flush`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn flush_async(&mut self) {
        self.create_eager_instances_async().await;

        self.evaluate_providers();
        self.create_eager_instances_async().await;
    }

    /// Returns a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// let a = cx.resolve::<A>();
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve<T: 'static + Send + Sync>(&mut self) -> T {
        self.resolve_with_name("")
    }

    /// Returns a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// let a = cx.resolve_with_name::<A>("a");
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_with_name<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> T {
        let name = name.into();
        match self.inner_resolve(name, Behaviour::CreateThenReturnSingletonOrTransient) {
            Resolved::SingletonOrTransient(instance) => instance,
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrTransient(definition) => {
                not_singleton_or_transient_panic(definition)
            }
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
    }

    /// Returns an optional [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and default name `""`.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient),
    /// this method will return `None`, otherwise it will return `Some`.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// assert!(cx.resolve_option::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_option<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.resolve_option_with_name("")
    }

    /// Returns an optional [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and name.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and name, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient),
    /// this method will return `None`, otherwise it will return `Some`.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// assert!(cx.resolve_option_with_name::<A>("a").is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_option_with_name<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<T> {
        match self.inner_resolve(name.into(), Behaviour::CreateThenReturnSingletonOrTransient) {
            Resolved::SingletonOrTransient(instance) => Some(instance),
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrTransient(_) => None,
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
    }

    /// Returns a collection of [`Singleton`](crate::Scope::Singleton) and [`Transient`](crate::Scope::Transient) instances of the given type.
    ///
    /// # Note
    ///
    /// This method will return a collection of [`Singleton`](crate::Scope::Singleton) and [`Transient`](crate::Scope::Transient),
    /// if some providers are [`SingleOwner`](crate::Scope::SingleOwner), they will not be contained in the collection.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient(name = "a")]
    /// fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "b")]
    /// fn B() -> i32 {
    ///     2
    /// }
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// assert_eq!(cx.resolve_by_type::<i32>().into_iter().sum::<i32>(), 3);
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_by_type<T: 'static + Send + Sync>(&mut self) -> Vec<T> {
        self.names::<T>()
            .into_iter()
            .filter_map(|name| self.resolve_option_with_name(name))
            .collect()
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn just_create<T: 'static + Send + Sync>(&mut self, name: Cow<'static, str>) {
        match self.inner_resolve::<T>(name, Behaviour::JustCreateAllScopeForEagerCreate) {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::SingletonOrTransient(_)
            | Resolved::NotSingletonOrTransient(_)
            | Resolved::NotSingletonOrSingleOwner(_) => {
                unreachable!()
            }
        }
    }

    /// Creates a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""` but does not return it.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// assert!(!cx.contains_single::<A>());
    /// cx.just_create_single::<A>();
    /// assert!(cx.contains_single::<A>());
    /// # }
    /// ```
    #[track_caller]
    pub fn just_create_single<T: 'static + Send + Sync>(&mut self) {
        self.just_create_single_with_name::<T>("");
    }

    /// Creates a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name but does not return it.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    /// assert!(!cx.contains_single_with_name::<A>("a"));
    /// cx.just_create_single_with_name::<A>("a");
    /// assert!(cx.contains_single_with_name::<A>("a"));
    /// # }
    /// ```
    #[track_caller]
    pub fn just_create_single_with_name<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self.inner_resolve::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner) {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrSingleOwner(definition) => {
                not_singleton_or_single_owner_panic(definition)
            }
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Try to create a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""` but does not return it.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    /// this method will return `false`, otherwise it will return `true`.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// #[Transient]
    /// struct B;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    ///
    /// assert!(!cx.contains_single::<A>());
    /// assert!(!cx.contains_single::<B>());
    ///
    /// assert!(cx.try_just_create_single::<A>());
    /// assert!(!cx.try_just_create_single::<B>());
    ///
    /// assert!(cx.contains_single::<A>());
    /// assert!(!cx.contains_single::<B>());
    /// # }
    /// ```
    #[track_caller]
    pub fn try_just_create_single<T: 'static + Send + Sync>(&mut self) -> bool {
        self.try_just_create_single_with_name::<T>("")
    }

    /// Try to create a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name but does not return it.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    /// this method will return `false`, otherwise it will return `true`.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// #[Transient(name = "b")]
    /// struct B;
    ///
    /// # fn main() {
    /// let mut cx = ApplicationContext::auto_register();
    ///
    /// assert!(!cx.contains_single_with_name::<A>("a"));
    /// assert!(!cx.contains_single_with_name::<B>("b"));
    ///
    /// assert!(cx.try_just_create_single_with_name::<A>("a"));
    /// assert!(!cx.try_just_create_single_with_name::<B>("b"));
    ///
    /// assert!(cx.contains_single_with_name::<A>("a"));
    /// assert!(!cx.contains_single_with_name::<B>("b"));
    /// # }
    /// ```
    #[track_caller]
    pub fn try_just_create_single_with_name<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        match self.inner_resolve::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner) {
            Resolved::NoReturn => true,
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrSingleOwner(_) => false,
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Try to create [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances based on the given type but does not return them.
    ///
    /// # Note
    ///
    /// This method will return a collection of booleans, if a provider is a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    /// the corresponding boolean value will be `true`, otherwise it will be `false`.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton, Transient};
    ///
    /// #[Singleton(name = "one")]
    /// fn One() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "two")]
    /// fn Two() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///
    ///     assert!(!cx.contains_single::<i32>());
    ///
    ///     let results = cx.try_just_create_singles_by_type::<i32>();
    ///
    ///     assert!(results.contains(&true));
    ///     assert!(results.contains(&false));
    ///
    ///     assert_eq!(cx.get_singles_by_type::<i32>(), vec![&1]);
    /// }
    /// ```
    #[track_caller]
    pub fn try_just_create_singles_by_type<T: 'static + Send + Sync>(&mut self) -> Vec<bool> {
        self.names::<T>()
            .into_iter()
            .map(|name| self.try_just_create_single_with_name::<T>(name))
            .collect()
    }

    /// Async version of [`ApplicationContext::resolve`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async)]
    /// struct A(i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert_eq!(cx.resolve_async::<i32>().await, 1);
    ///     assert!(cx.resolve_option_async::<A>().await.is_some());
    /// }
    /// ```
    pub async fn resolve_async<T: 'static + Send + Sync>(&mut self) -> T {
        self.resolve_with_name_async("").await
    }

    /// Async version of [`ApplicationContext::resolve_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient(name = "a")]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async, name = "A")]
    /// struct A(#[di(name = "a")] i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert_eq!(cx.resolve_with_name_async::<i32>("a").await, 1);
    ///     assert!(cx.resolve_option_with_name_async::<A>("A").await.is_some());
    /// }
    /// ```
    pub async fn resolve_with_name_async<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> T {
        match self
            .inner_resolve_async(name.into(), Behaviour::CreateThenReturnSingletonOrTransient)
            .await
        {
            Resolved::SingletonOrTransient(instance) => instance,
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrTransient(definition) => {
                not_singleton_or_transient_panic(definition)
            }
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
    }

    /// Async version of [`ApplicationContext::resolve_option`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async)]
    /// struct A(i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert_eq!(cx.resolve_async::<i32>().await, 1);
    ///     assert!(cx.resolve_option_async::<A>().await.is_some());
    /// }
    /// ```
    pub async fn resolve_option_async<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.resolve_option_with_name_async("").await
    }

    /// Async version of [`ApplicationContext::resolve_option_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient(name = "a")]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async, name = "A")]
    /// struct A(#[di(name = "a")] i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert_eq!(cx.resolve_with_name_async::<i32>("a").await, 1);
    ///     assert!(cx.resolve_option_with_name_async::<A>("A").await.is_some());
    /// }
    /// ```
    pub async fn resolve_option_with_name_async<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<T> {
        match self
            .inner_resolve_async(name.into(), Behaviour::CreateThenReturnSingletonOrTransient)
            .await
        {
            Resolved::SingletonOrTransient(instance) => Some(instance),
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrTransient(_) => None,
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
    }

    /// Async version of [`ApplicationContext::resolve_by_type`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient(name = "a")]
    /// async fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "b")]
    /// async fn B() -> i32 {
    ///     2
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert_eq!(
    ///         cx.resolve_by_type_async::<i32>()
    ///             .await
    ///             .into_iter()
    ///             .sum::<i32>(),
    ///         3
    ///     );
    /// }
    /// ```
    pub async fn resolve_by_type_async<T: 'static + Send + Sync>(&mut self) -> Vec<T> {
        let names = self.names::<T>();

        let mut instances = Vec::with_capacity(names.len());

        for name in names {
            if let Some(instance) = self.resolve_option_with_name_async(name).await {
                instances.push(instance);
            }
        }

        instances
    }

    #[doc(hidden)]
    pub async fn just_create_async<T: 'static + Send + Sync>(&mut self, name: Cow<'static, str>) {
        match self
            .inner_resolve_async::<T>(name, Behaviour::JustCreateAllScopeForEagerCreate)
            .await
        {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::SingletonOrTransient(_)
            | Resolved::NotSingletonOrTransient(_)
            | Resolved::NotSingletonOrSingleOwner(_) => {
                unreachable!()
            }
        }
    }

    /// Async version of [`ApplicationContext::just_create_single`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async)]
    /// struct A;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert!(!cx.contains_single::<A>());
    ///     cx.just_create_single_async::<A>().await;
    ///     assert!(cx.contains_single::<A>());
    /// }
    /// ```
    pub async fn just_create_single_async<T: 'static + Send + Sync>(&mut self) {
        self.just_create_single_with_name_async::<T>("").await;
    }

    /// Async version of [`ApplicationContext::just_create_single_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async, name = "a")]
    /// struct A;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///     assert!(!cx.contains_single_with_name::<A>("a"));
    ///     cx.just_create_single_with_name_async::<A>("a").await;
    ///     assert!(cx.contains_single_with_name::<A>("a"));
    /// }
    /// ```
    pub async fn just_create_single_with_name_async<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self
            .inner_resolve_async::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner)
            .await
        {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrSingleOwner(definition) => {
                not_singleton_or_single_owner_panic(definition)
            }
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Async version of [`ApplicationContext::try_just_create_single`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async)]
    /// struct A;
    ///
    /// #[Transient(async)]
    /// struct B;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///
    ///     assert!(!cx.contains_single::<A>());
    ///     assert!(!cx.contains_single::<B>());
    ///
    ///     assert!(cx.try_just_create_single_async::<A>().await);
    ///     assert!(!cx.try_just_create_single_async::<B>().await);
    ///
    ///     assert!(cx.contains_single::<A>());
    ///     assert!(!cx.contains_single::<B>());
    /// }
    /// ```
    pub async fn try_just_create_single_async<T: 'static + Send + Sync>(&mut self) -> bool {
        self.try_just_create_single_with_name_async::<T>("").await
    }

    /// Async version of [`ApplicationContext::try_just_create_single_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async, name = "a")]
    /// struct A;
    ///
    /// #[Transient(async, name = "b")]
    /// struct B;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///
    ///     assert!(!cx.contains_single_with_name::<A>("a"));
    ///     assert!(!cx.contains_single_with_name::<B>("b"));
    ///
    ///     assert!(cx.try_just_create_single_with_name_async::<A>("a").await);
    ///     assert!(!cx.try_just_create_single_with_name_async::<B>("b").await);
    ///
    ///     assert!(cx.contains_single_with_name::<A>("a"));
    ///     assert!(!cx.contains_single_with_name::<B>("b"));
    /// }
    /// ```
    pub async fn try_just_create_single_with_name_async<T: 'static + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        match self
            .inner_resolve_async::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner)
            .await
        {
            Resolved::NoReturn => true,
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrSingleOwner(_) => false,
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Async version of [`ApplicationContext::try_just_create_singles_by_type`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton, Transient};
    ///
    /// #[Singleton(name = "one")]
    /// async fn One() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "two")]
    /// async fn Two() -> i32 {
    ///     2
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = ApplicationContext::auto_register();
    ///
    ///     assert!(!cx.contains_single::<i32>());
    ///
    ///     let results = cx.try_just_create_singles_by_type_async::<i32>().await;
    ///
    ///     assert!(results.contains(&true));
    ///     assert!(results.contains(&false));
    ///
    ///     assert_eq!(cx.get_singles_by_type::<i32>(), vec![&1]);
    /// }
    /// ```
    pub async fn try_just_create_singles_by_type_async<T: 'static + Send + Sync>(
        &mut self,
    ) -> Vec<bool> {
        let names = self.names::<T>();
        let mut results = Vec::with_capacity(names.len());

        for name in names {
            let result = self.try_just_create_single_with_name_async::<T>(name).await;
            results.push(result);
        }

        results
    }

    /// Returns true if the context contains a provider for the specified type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.contains_provider::<A>());
    /// # }
    /// ```
    pub fn contains_provider<T: 'static>(&self) -> bool {
        self.contains_provider_with_name::<T>("")
    }

    /// Returns true if the context contains a provider for the specified type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.contains_provider_with_name::<A>("a"));
    /// # }
    /// ```
    pub fn contains_provider_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        let key = Key::new::<T>(name.into());
        self.provider_registry.contains(&key)
    }

    /// Returns a reference to an provider based on the given type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.get_provider::<A>().is_some());
    /// # }
    /// ```
    pub fn get_provider<T: 'static + Send + Sync>(&self) -> Option<&Provider<T>> {
        self.get_provider_with_name("")
    }

    /// Returns a reference to an provider based on the given type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.get_provider_with_name::<A>("a").is_some());
    /// # }
    /// ```
    pub fn get_provider_with_name<T: 'static + Send + Sync>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&Provider<T>> {
        let key = Key::new::<T>(name.into());
        self.provider_registry.get(&key)
    }

    /// Returns a collection of references to providers based on the given type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Transient};
    ///
    /// #[Transient(name = "a")]
    /// fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "b")]
    /// fn B() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let cx = ApplicationContext::auto_register();
    ///     assert_eq!(cx.get_providers_by_type::<i32>().len(), 2);
    /// }
    /// ```
    pub fn get_providers_by_type<T: 'static + Send + Sync>(&self) -> Vec<&Provider<T>> {
        let type_id = TypeId::of::<T>();

        self.provider_registry()
            .iter()
            .filter(|(key, _)| key.ty.id == type_id)
            .filter_map(|(_, provider)| provider.as_provider())
            .collect()
    }

    /// Returns true if the context contains a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance for the specified type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.contains_single::<A>());
    /// # }
    /// ```
    pub fn contains_single<T: 'static>(&self) -> bool {
        self.contains_single_with_name::<T>("")
    }

    /// Returns true if the context contains a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance for the specified type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.contains_single_with_name::<A>("a"));
    /// # }
    /// ```
    pub fn contains_single_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        let key = Key::new::<T>(name.into());
        self.single_registry.contains(&key)
    }

    /// Returns a reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if no single instance is registered for the given type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// let a = cx.get_single::<A>();
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn get_single<T: 'static>(&self) -> &T {
        self.get_single_with_name("")
    }

    /// Returns a reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if no single instance is registered for the given type and name.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// let a = cx.get_single_with_name::<A>("a");
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn get_single_with_name<T: 'static>(&self, name: impl Into<Cow<'static, str>>) -> &T {
        let key = Key::new::<T>(name.into());
        // println!("keys: {:?}", self.single_registry.keys());
        self.single_registry
            .get_ref(&key)
            .unwrap_or_else(|| panic!("no instance registered for: {:?}", key))
    }

    /// Returns an optional reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.get_single_option::<A>().is_some());
    /// # }
    /// ```
    pub fn get_single_option<T: 'static>(&self) -> Option<&T> {
        self.get_single_option_with_name("")
    }

    /// Returns an optional reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = ApplicationContext::auto_register();
    /// assert!(cx.get_single_option_with_name::<A>("a").is_some());
    /// # }
    /// ```
    pub fn get_single_option_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&T> {
        let key = Key::new::<T>(name.into());
        self.single_registry.get_ref(&key)
    }

    /// Returns a collection of references to [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances based on the given type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{ApplicationContext, Singleton};
    ///
    /// #[Singleton(eager_create, name = "a")]
    /// fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Singleton(eager_create, name = "b")]
    /// fn B() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let cx = ApplicationContext::auto_register();
    ///     assert_eq!(cx.get_singles_by_type::<i32>().into_iter().sum::<i32>(), 3);
    /// }
    /// ```
    pub fn get_singles_by_type<T: 'static>(&self) -> Vec<&T> {
        let type_id = TypeId::of::<T>();

        self.single_registry()
            .iter()
            .filter(|(key, _)| key.ty.id == type_id)
            .filter_map(|(_, instance)| instance.as_single())
            .map(|instance| instance.get_ref())
            .collect()
    }
}

impl ApplicationContext {
    #[track_caller]
    fn load_provider(&mut self, eager_create: bool, provider: DynProvider) {
        let definition = provider.definition();
        let need_eager_create = self.eager_create || eager_create || provider.eager_create();

        let allow_all_scope = !self.allow_only_single_eager_create;

        let allow_only_single_and_it_is_single = matches!(
            (self.allow_only_single_eager_create, definition.scope),
            (true, Scope::Singleton) | (true, Scope::SingleOwner)
        );

        let allow_eager_create = allow_all_scope || allow_only_single_and_it_is_single;

        if need_eager_create && allow_eager_create {
            self.eager_create_functions
                .push((definition.clone(), provider.eager_create_function()));
        }

        self.provider_registry.insert(provider, self.allow_override);
    }

    #[track_caller]
    fn load_providers(&mut self, eager_create: bool, providers: Vec<DynProvider>) {
        if providers.is_empty() {
            return;
        }

        let providers = flatten(providers, DynProvider::binding_providers);

        providers.into_iter().for_each(|provider| {
            if provider.condition().is_some() {
                self.conditional_providers.push((eager_create, provider));
                return;
            }

            self.load_provider(eager_create, provider);
        });
    }

    fn unload_providers(&mut self, providers: Vec<DynProvider>) {
        if providers.is_empty() {
            return;
        }

        let providers = flatten(providers, DynProvider::binding_providers);

        providers.into_iter().for_each(|provider| {
            let key = provider.key();
            self.provider_registry.remove(key);
            self.single_registry.remove(key);
        });
    }

    #[track_caller]
    fn create_eager_instances(&mut self) {
        if self.eager_create_functions.is_empty() {
            return;
        }

        self.eager_create_functions.reverse();

        while let Some((definition, eager_create_function)) = self.eager_create_functions.pop() {
            match eager_create_function {
                EagerCreateFunction::Async(_) => {
                    panic!(
                        "unable to call an async eager create function in a sync context for: {:?}

please use instead:
1. ApplicationContext::create_async(modules).await
2. ApplicationContext::auto_register_async().await
3. ApplicationContextOptions::create_async(options, modules).await
4. ApplicationContextOptions::auto_register_async(options).await
",
                        definition
                    )
                }
                EagerCreateFunction::Sync(eager_create_function) => {
                    eager_create_function(self, definition.key.name)
                }
                EagerCreateFunction::None => unreachable!(),
            }
        }
    }

    async fn create_eager_instances_async(&mut self) {
        if self.eager_create_functions.is_empty() {
            return;
        }

        self.eager_create_functions.reverse();

        while let Some((definition, eager_create_function)) = self.eager_create_functions.pop() {
            match eager_create_function {
                EagerCreateFunction::Async(eager_create_function) => {
                    eager_create_function(self, definition.key.name).await
                }
                EagerCreateFunction::Sync(eager_create_function) => {
                    eager_create_function(self, definition.key.name)
                }
                EagerCreateFunction::None => unreachable!(),
            }
        }
    }

    #[track_caller]
    fn evaluate_providers(&mut self) {
        if self.conditional_providers.is_empty() {
            return;
        }

        self.conditional_providers.reverse();

        while let Some((eager_create, provider)) = self.conditional_providers.pop() {
            let evaluate = provider.condition().expect("unreachable: a provider in `conditional_providers`, its `condition()` method must return `Some(_)`");

            if evaluate(self) {
                self.load_provider(eager_create, provider);
            } else {
                #[cfg(feature = "tracing")]
                tracing::warn!("(×) condition not met: {:?}", provider.definition());
            }
        }
    }

    fn before_resolve<T: 'static + Send + Sync>(
        &mut self,
        name: Cow<'static, str>,
        behaviour: Behaviour,
    ) -> Result<Resolved<T>, Holder<'_, T>> {
        let key = Key::new::<T>(name);

        let Some(provider) = self.provider_registry.get::<T>(&key) else {
            return Ok(Resolved::NotFoundProvider(key));
        };

        let definition = provider.definition();

        if self.single_registry.contains(&key) {
            return Ok(match behaviour {
                Behaviour::CreateThenReturnSingletonOrTransient => {
                    match self.single_registry.get_owned::<T>(&key) {
                        Some(instance) => Resolved::SingletonOrTransient(instance),
                        None => Resolved::NotSingletonOrTransient(definition.clone()),
                    }
                }
                Behaviour::JustCreateAllScopeForEagerCreate
                | Behaviour::JustCreateSingletonOrSingleOwner => Resolved::NoReturn,
            });
        }

        match (definition.scope, behaviour) {
            (Scope::Transient, Behaviour::JustCreateSingletonOrSingleOwner) => {
                return Ok(Resolved::NotSingletonOrSingleOwner(definition.clone()))
            }
            (Scope::SingleOwner, Behaviour::CreateThenReturnSingletonOrTransient) => {
                return Ok(Resolved::NotSingletonOrTransient(definition.clone()))
            }
            _ => {}
        }

        let constructor = provider.constructor();
        let clone_instance = provider.clone_instance();

        Err(Holder {
            key,
            constructor,
            clone_instance,
            definition,
        })
    }

    fn after_resolve<T: 'static + Send + Sync>(
        &mut self,
        key: Key,
        behaviour: Behaviour,
        scope: Scope,
        instance: T,
        clone_instance: Option<fn(&T) -> T>,
    ) -> Resolved<T> {
        match (scope, behaviour) {
            // Singleton
            (Scope::Singleton, Behaviour::CreateThenReturnSingletonOrTransient) => {
                self.single_registry.insert(
                    key,
                    Single::new((clone_instance.unwrap())(&instance), clone_instance).into(),
                );

                Resolved::SingletonOrTransient(instance)
            }
            (Scope::Singleton, Behaviour::JustCreateAllScopeForEagerCreate)
            | (Scope::Singleton, Behaviour::JustCreateSingletonOrSingleOwner) => {
                self.single_registry
                    .insert(key, Single::new(instance, clone_instance).into());

                Resolved::NoReturn
            }
            // Transient
            (Scope::Transient, Behaviour::CreateThenReturnSingletonOrTransient) => {
                Resolved::SingletonOrTransient(instance)
            }
            (Scope::Transient, Behaviour::JustCreateAllScopeForEagerCreate) => Resolved::NoReturn,
            (Scope::Transient, Behaviour::JustCreateSingletonOrSingleOwner) => unreachable!(),
            // SingleOwner
            (Scope::SingleOwner, Behaviour::CreateThenReturnSingletonOrTransient) => unreachable!(),
            (Scope::SingleOwner, Behaviour::JustCreateAllScopeForEagerCreate)
            | (Scope::SingleOwner, Behaviour::JustCreateSingletonOrSingleOwner) => {
                self.single_registry
                    .insert(key, Single::new(instance, None).into());

                Resolved::NoReturn
            }
        }
    }

    #[track_caller]
    fn inner_resolve<T: 'static + Send + Sync>(
        &mut self,
        name: Cow<'static, str>,
        behaviour: Behaviour,
    ) -> Resolved<T> {
        let Holder {
            key,
            constructor,
            clone_instance,
            definition,
        } = match self.before_resolve(name, behaviour) {
            Ok(o) => return o,
            Err(e) => e,
        };

        let scope = definition.scope;

        let instance = match constructor {
            Constructor::Async(_) => {
                panic!(
                    "unable to call an async constructor in a sync context for: {:?}

please check all the references to the above type, there are 3 scenarios that will be referenced:
1. use `ApplicationContext::resolve_xxx::<Type>(cx)` to get instances of the type, change to `ApplicationContext::resolve_xxx_async::<Type>(cx).await`.
2. use `yyy: Type` as a field of a struct, or a field of a variant of a enum, use `#[Singleton(async)]`, `#[Transient(async)]` or `#[SingleOwner(async)]` on the struct or enum.
3. use `zzz: Type` as a argument of a function, add the `async` keyword to the function.
",
                    definition
                )
            }
            Constructor::Sync(constructor) => self.resolve_instance(key.clone(), constructor),
        };

        self.after_resolve(key, behaviour, scope, instance, clone_instance)
    }

    async fn inner_resolve_async<T: 'static + Send + Sync>(
        &mut self,
        name: Cow<'static, str>,
        behaviour: Behaviour,
    ) -> Resolved<T> {
        let Holder {
            key,
            constructor,
            clone_instance,
            definition,
        } = match self.before_resolve(name, behaviour) {
            Ok(o) => return o,
            Err(e) => e,
        };

        let scope = definition.scope;

        let instance = {
            let key = key.clone();

            match constructor {
                Constructor::Async(constructor) => {
                    self.resolve_instance_async(key, constructor).await
                }
                Constructor::Sync(constructor) => self.resolve_instance(key, constructor),
            }
        };

        self.after_resolve(key, behaviour, scope, instance, clone_instance)
    }

    #[track_caller]
    fn resolve_instance<T: 'static + Send + Sync>(
        &mut self,
        key: Key,
        constructor: Arc<dyn Fn(&mut ApplicationContext) -> T>,
    ) -> T {
        self.dependency_chain.push(key);
        let instance = constructor(self);
        self.dependency_chain.pop();
        instance
    }

    #[allow(clippy::type_complexity)]
    async fn resolve_instance_async<T: 'static>(
        &mut self,
        key: Key,
        constructor: Arc<dyn for<'a> Fn(&'a mut ApplicationContext) -> BoxFuture<'a, T> + Send + Sync>,
    ) -> T {
        self.dependency_chain.push(key);
        let instance = constructor(self).await;
        self.dependency_chain.pop();
        instance
    }

    fn names<T: 'static>(&self) -> Vec<Cow<'static, str>> {
        let type_id = TypeId::of::<T>();

        self.provider_registry()
            .keys()
            .filter(|&key| key.ty.id == type_id)
            .map(|key| key.name.clone())
            .collect()
    }
}





/// =======================================================================================================

#[derive(Clone, Copy)]
enum Behaviour {
    CreateThenReturnSingletonOrTransient,
    JustCreateAllScopeForEagerCreate,
    JustCreateSingletonOrSingleOwner,
}

enum Resolved<T> {
    NotFoundProvider(Key),

    SingletonOrTransient(T),
    NotSingletonOrTransient(Definition),

    NoReturn,

    NotSingletonOrSingleOwner(Definition),
}

struct Holder<'a, T: Send + Sync> {
    key: Key,
    constructor: Constructor<T>,
    clone_instance: Option<fn(&T) -> T>,
    definition: &'a Definition,
}

#[inline(always)]
fn no_provider_panic(key: Key) -> ! {
    panic!("no provider registered for: {:?}", key)
}

#[inline(always)]
fn not_singleton_or_single_owner_panic(definition: Definition) -> ! {
    panic!(
        "registered provider is not `Singleton` or `SingleOwner` for: {:?}",
        definition
    )
}

#[inline(always)]
fn not_singleton_or_transient_panic(definition: Definition) -> ! {
    panic!(
        "registered provider is not `Singleton` or `Transient` for: {:?}",
        definition
    )
}

fn flatten<T, F>(mut unresolved: Vec<T>, get_sublist: F) -> Vec<T>
where
    F: Fn(&mut T) -> Option<Vec<T>>,
{
    debug_assert!(!unresolved.is_empty());

    let mut resolved = Vec::with_capacity(unresolved.len());

    unresolved.reverse();

    while let Some(mut element) = unresolved.pop() {
        match get_sublist(&mut element) {
            Some(mut sublist) if !sublist.is_empty() => {
                sublist.reverse();
                unresolved.append(&mut sublist);
            }
            _ => {}
        }

        resolved.push(element);
    }

    resolved
}

/// Options and flags which can be used to configure how a context is created.
///
/// This builder expose the ability to configure how a [`ApplicationContext`] is created.
/// The [`ApplicationContext::create`] and [`ApplicationContext::auto_register`] methods are aliases
/// for commonly used options using this builder.
///
/// Generally speaking, when using `ApplicationContextOptions`, you'll first call [`ApplicationContextOptions::default`],
/// then chain calls to methods to set each option, then call [`ApplicationContextOptions::create`], paasing the modules you've built,
/// or call [`ApplicationContextOptions::auto_register`]. This will give you a [`ApplicationContext`].
///
/// # Example
///
/// Creating a context with a module:
///
/// ```rust
/// use rudi::{modules, ApplicationContext, ApplicationContextOptions, DynProvider, Module};
///
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn providers() -> Vec<DynProvider> {
///         vec![]
///     }
/// }
///
/// # fn main() {
/// let _cx: ApplicationContext = ApplicationContextOptions::default().create(modules![MyModule]);
/// # }
/// ```
///
/// Creating a context with [`AutoRegisterModule`]:
///
/// ```rust
/// use rudi::{modules, AutoRegisterModule, ApplicationContext, ApplicationContextOptions};
///
/// # fn main() {
/// let _cx: ApplicationContext = ApplicationContextOptions::default().create(modules![AutoRegisterModule]);
/// // or use simpler method
/// // let _cx: ApplicationContext = ApplicationContextOptions::default().auto_register();
/// # }
/// ```
///
/// Creating a context with both options:
///
/// ```rust
/// use rudi::{modules, AutoRegisterModule, ApplicationContext, ApplicationContextOptions};
///
/// # fn main() {
/// let _cx: ApplicationContext = ApplicationContextOptions::default()
///     .allow_override(true)
///     .allow_only_single_eager_create(true)
///     .eager_create(false)
///     .singleton(42)
///     .singleton_with_name("Hello", "str_1")
///     .singleton_with_name("World", "str_2")
///     .create(modules![AutoRegisterModule]);
/// # }
/// ```
///
/// [`AutoRegisterModule`]: crate::AutoRegisterModule
pub struct ApplicationContextOptions {
    allow_override: bool,
    allow_only_single_eager_create: bool,
    eager_create: bool,
    providers: Vec<DynProvider>,
    singles: Vec<DynSingle>,
}

impl Default for ApplicationContextOptions {
    fn default() -> Self {
        Self {
            allow_override: true,
            allow_only_single_eager_create: true,
            eager_create: Default::default(),
            providers: Default::default(),
            singles: Default::default(),
        }
    }
}

impl ApplicationContextOptions {
    /// Sets the option for whether the context should allow overriding existing providers.
    ///
    /// This option, when true, allows a provider to override an existing provider with the same key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default()
    ///     .allow_override(true)
    ///     .create(modules![]);
    /// assert!(cx.allow_override());
    /// # }
    /// ```
    pub fn allow_override(mut self, allow_override: bool) -> Self {
        self.allow_override = allow_override;
        self
    }

    /// Sets the option for whether the context should only eagerly create [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances.
    ///
    /// This option, when true, will only eagerly create instances for [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) providers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default()
    ///     .allow_only_single_eager_create(false)
    ///     .create(modules![]);
    /// assert!(!cx.allow_only_single_eager_create());
    /// # }
    /// ```
    pub fn allow_only_single_eager_create(mut self, allow_only_single_eager_create: bool) -> Self {
        self.allow_only_single_eager_create = allow_only_single_eager_create;
        self
    }

    /// Sets the option for whether the context should eagerly create instances.
    ///
    /// This option, when true, will eagerly create instances for all providers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default()
    ///     .eager_create(false)
    ///     .create(modules![]);
    /// assert!(!cx.eager_create());
    /// # }
    /// ```
    pub fn eager_create(mut self, eager_create: bool) -> Self {
        self.eager_create = eager_create;
        self
    }

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default().singleton(42).create(modules![]);
    /// assert_eq!(cx.get_single::<i32>(), &42);
    /// # }
    /// ```
    pub fn singleton<T>(self, instance: T) -> Self
    where
        T: 'static + Clone + Send + Sync,
    {
        self.singleton_with_name(instance, "")
    }

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default()
    ///     .singleton_with_name(1, "one")
    ///     .singleton_with_name(2, "two")
    ///     .create(modules![]);
    ///
    /// assert_eq!(cx.get_single_with_name::<i32>("one"), &1);
    /// assert_eq!(cx.get_single_with_name::<i32>("two"), &2);
    /// # }
    /// ```
    pub fn singleton_with_name<T, N>(mut self, instance: T, name: N) -> Self
    where
        T: 'static + Clone + Send + Sync,
        N: Into<Cow<'static, str>>,
    {
        let provider = Provider::<T>::never_construct(name.into(), Scope::Singleton).into();
        let single = Single::new(instance, Some(Clone::clone)).into();

        self.providers.push(provider);
        self.singles.push(single);

        self
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default()
    ///     .single_owner(NotClone(42))
    ///     .create(modules![]);
    /// assert_eq!(cx.get_single::<NotClone>(), &NotClone(42));
    /// # }
    /// ```
    pub fn single_owner<T>(self, instance: T) -> Self
    where
        T: 'static + Send + Sync,
    {
        self.single_owner_with_name(instance, "")
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, ApplicationContext, ApplicationContextOptions};
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let cx: ApplicationContext = ApplicationContextOptions::default()
    ///     .single_owner_with_name(NotClone(1), "one")
    ///     .single_owner_with_name(NotClone(2), "two")
    ///     .create(modules![]);
    ///
    /// assert_eq!(cx.get_single_with_name::<NotClone>("one"), &NotClone(1));
    /// assert_eq!(cx.get_single_with_name::<NotClone>("two"), &NotClone(2));
    /// # }
    /// ```
    pub fn single_owner_with_name<T, N>(mut self, instance: T, name: N) -> Self
    where
        T: 'static + Send + Sync,
        N: Into<Cow<'static, str>>,
    {
        let provider = Provider::<T>::never_construct(name.into(), Scope::SingleOwner).into();
        let single = Single::new(instance, None).into();

        self.providers.push(provider);
        self.singles.push(single);

        self
    }

    #[track_caller]
    fn inner_create<F>(self, init: F) -> ApplicationContext
    where
        F: FnOnce(&mut ApplicationContext),
    {
        let ApplicationContextOptions {
            allow_override,
            allow_only_single_eager_create,
            eager_create,
            providers,
            singles,
        } = self;

        let mut cx = ApplicationContext {
            allow_override,
            allow_only_single_eager_create,
            eager_create,
            ..Default::default()
        };

        if !providers.is_empty() {
            providers
                .into_iter()
                .zip(singles)
                .for_each(|(provider, single)| {
                    let key = provider.key().clone();
                    cx.provider_registry.insert(provider, allow_override);
                    cx.single_registry.insert(key, single);
                });
        }

        init(&mut cx);

        cx
    }

    #[track_caller]
    fn inner_create_with_modules(self, modules: Vec<ResolveModule>) -> ApplicationContext {
        self.inner_create(|cx| cx.load_modules(modules))
    }

    #[track_caller]
    fn inner_create_with_auto(self) -> ApplicationContext {
        self.inner_create(|cx| {
            let module = ResolveModule::new::<AutoRegisterModule>();
            cx.loaded_modules.push(module.ty());
            cx.load_providers(module.eager_create(), module.providers())
        })
    }

    /// Creates a new context with the given modules.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{components, modules, ApplicationContext, ApplicationContextOptions, DynProvider, Module, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers() -> Vec<DynProvider> {
    ///         components![A]
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let mut cx: ApplicationContext = ApplicationContextOptions::default().create(modules![MyModule]);
    /// assert!(cx.resolve_option::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn create(self, modules: Vec<ResolveModule>) -> ApplicationContext {
        let mut cx = self.inner_create_with_modules(modules);
        cx.flush();
        cx
    }

    /// Creates a new context with the [`AutoRegisterModule`].
    ///
    /// Same as `ApplicationContextOptions::default().create(modules![AutoRegisterModule])`.
    ///
    /// See [`ApplicationContextOptions::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// [`AutoRegisterModule`]: crate::AutoRegisterModule
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[track_caller]
    pub fn auto_register(self) -> ApplicationContext {
        let mut cx = self.inner_create_with_auto();
        cx.flush();
        cx
    }

    /// Async version of [`ApplicationContextOptions::create`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ApplicationContextOptions::create`].
    ///
    /// See [`ApplicationContextOptions::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn create_async(self, modules: Vec<ResolveModule>) -> ApplicationContext {
        let mut cx = self.inner_create_with_modules(modules);
        cx.flush_async().await;
        cx
    }

    /// Async version of [`ApplicationContextOptions::auto_register`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ApplicationContextOptions::auto_register`].
    ///
    /// See [`ApplicationContextOptions::auto_register`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](ApplicationContext::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    pub async fn auto_register_async(self) -> ApplicationContext {
        let mut cx = self.inner_create_with_auto();
        cx.flush_async().await;
        cx
    }
}

#[derive(Default, Clone)]
struct DependencyChain {
    stack: Vec<Key>,
}

impl DependencyChain {
    fn push(&mut self, key: Key) {
        let already_contains = self.stack.contains(&key);
        self.stack.push(key);

        if already_contains {
            let key = self.stack.last().unwrap();

            let mut buf = String::with_capacity(1024);
            buf.push('[');
            buf.push('\n');

            self.stack.iter().for_each(|k| {
                if key == k {
                    buf.push_str(" --> ")
                } else {
                    buf.push_str("  |  ")
                }

                buf.push_str(&format!("{:?}", k));
                buf.push('\n');
            });

            buf.push(']');

            panic!("circular dependency detected: {}", buf);
        }
    }

    fn pop(&mut self) {
        self.stack.pop();
    }
}

/// Represents a type.
#[derive(Clone, Copy, Debug)]
pub struct Type {
    /// The name of the type.
    pub name: &'static str,
    /// The unique identifier of the type.
    pub id: TypeId,
}

impl Type {
    pub(crate) fn new<T: 'static>() -> Type {
        Type {
            name: any::type_name::<T>(),
            id: TypeId::of::<T>(),
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Type {}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// An owned dynamically typed [`Future`] for use in cases where you can't
/// statically type your result or need to add some indirection.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

impl<T: ?Sized + Send + Sync> FutureExt for T where T: Future {}

/// An extension trait for `Future`s that provides a convenient adapter.
pub trait FutureExt: Future {
    /// Wrap the future in a Box, pinning it.
    fn boxed<'a>(self) -> BoxFuture<'a, Self::Output>
    where
        Self: Sized + 'static + Send + Sync,
    {
        Box::pin(self)
    }
}

#[derive(Default, Clone)]
pub(crate) struct SingleRegistry {
    registry: HashMap<Key, DynSingle>,
}

impl SingleRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynSingle> {
        &self.registry
    }

    pub(crate) fn insert(&mut self, key: Key, single: DynSingle) {
        // There is no need to check the value of `allow_override` here,
        // because when inserting a provider and a single with the same key into the context,
        // the provider must be inserted first, followed by the single,
        // and the checking of `allow_override` has already been done when the provider is inserted.
        self.registry.insert(key, single);
    }

    pub(crate) fn get_owned<T: 'static>(&self, key: &Key) -> Option<T> {
        self.registry.get(key)?.as_single::<T>()?.get_owned()
    }

    pub(crate) fn get_ref<T: 'static>(&self, key: &Key) -> Option<&T> {
        Some(self.registry.get(key)?.as_single::<T>()?.get_ref())
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynSingle> {
        self.registry.remove(key)
    }

    pub(crate) fn keys(&self) -> Keys<Key, DynSingle> {
        self.registry.keys()
    }
}

#[derive(Default, Clone)]
pub(crate) struct ProviderRegistry {
    registry: HashMap<Key, DynProvider>,
}

impl ProviderRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynProvider> {
        &self.registry
    }

    #[track_caller]
    pub(crate) fn insert(&mut self, provider: DynProvider, allow_override: bool) {
        let definition = provider.definition();
        let key = provider.key().clone();

        if !self.registry.contains_key(&key) {
            #[cfg(feature = "tracing")]
            tracing::debug!("(+) insert new: {:?}", definition);
        } else if allow_override {
            #[cfg(feature = "tracing")]
            tracing::warn!("(!) override by `key`: {:?}", definition);
        } else {
            panic!(
                "already existing a provider with the same `key`: {:?}",
                definition
            );
        }

        self.registry.insert(key, provider);
    }

    pub(crate) fn get<T: 'static + Send + Sync>(&self, key: &Key) -> Option<&Provider<T>> {
        self.registry.get(key)?.as_provider()
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynProvider> {
        self.registry.remove(key)
    }
}

/// Represents a module.
///
/// # Example
///
/// ```rust
/// use rudi::{
///     modules, providers, singleton, transient, ApplicationContext, DynProvider, Module, ResolveModule,
/// };
///
/// struct Module1;
///
/// impl Module for Module1 {
///     fn eager_create() -> bool {
///         true
///     }
///
///     fn providers() -> Vec<DynProvider> {
///         providers![singleton(|_| "Hello").name("1")]
///     }
/// }
///
/// struct Module2;
///
/// impl Module for Module2 {
///     fn submodules() -> Option<Vec<ResolveModule>> {
///         Some(modules![Module1])
///     }
///
///     fn providers() -> Vec<DynProvider> {
///         providers![transient(|_| "World").name("2")]
///     }
/// }
///
/// # fn main() {
/// let mut cx = ApplicationContext::create(modules![Module2]);
/// let mut a = cx.resolve_by_type::<&'static str>();
/// a.sort();
/// assert!(format!("{:?}", a) == *r#"["Hello", "World"]"#);
/// # }
/// ```
pub trait Module {
    /// Whether the providers included in the module should be created eagerly, default is false.
    fn eager_create() -> bool {
        false
    }

    /// Included submodules, default is None.
    fn submodules() -> Option<Vec<ResolveModule>> {
        None
    }

    /// Included providers.
    fn providers() -> Vec<DynProvider>;
}

/// A type representing a Module, converted from a type that implements [`Module`].
pub struct ResolveModule {
    ty: Type,
    eager_create: bool,
    submodules: Option<Vec<ResolveModule>>,
    providers: Vec<DynProvider>,
}

impl ResolveModule {
    /// Create a [`ResolveModule`] from a type that implements [`Module`].
    pub fn new<T: Module + 'static>() -> Self {
        Self {
            ty: Type::new::<T>(),
            eager_create: T::eager_create(),
            submodules: T::submodules(),
            providers: T::providers(),
        }
    }

    /// Represents the type that is converted to a ResolveModule.
    pub fn ty(&self) -> Type {
        self.ty
    }

    /// Whether the providers included in the module should be created eagerly.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    pub(crate) fn submodules(&mut self) -> Option<Vec<ResolveModule>> {
        self.submodules.take()
    }

    pub(crate) fn providers(self) -> Vec<DynProvider> {
        self.providers
    }
}

/// Represents a unique key for a provider.
#[derive(Clone, Debug)]
pub struct Key {
    /// The name of the provider.
    pub name: Cow<'static, str>,
    /// The type of the provider generic.
    pub ty: Type,
}

impl Key {
    pub(crate) fn new<T: 'static>(name: Cow<'static, str>) -> Self {
        Self {
            name,
            ty: Type::new::<T>(),
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.name == other.name
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ty.cmp(&other.ty) {
            Ordering::Equal => {}
            ord => return ord,
        }
        self.name.cmp(&other.name)
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.name.hash(state);
    }
}

/// Represents a definition of a provider.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Definition {
    /// The unique key of the provider.
    pub key: Key,
    /// The origin type of the provider.
    ///
    /// When the following methods are called, current definition represents the
    /// return type of the method, and this field represents the parameter type of the method:
    /// - [`SingletonProvider::bind`](crate::SingletonProvider::bind)
    /// - [`TransientProvider::bind`](crate::TransientProvider::bind)
    /// - [`SingleOwnerProvider::bind`](crate::SingleOwnerProvider::bind)
    /// - [`SingletonAsyncProvider::bind`](crate::SingletonAsyncProvider::bind)
    /// - [`TransientAsyncProvider::bind`](crate::TransientAsyncProvider::bind)
    /// - [`SingleOwnerAsyncProvider::bind`](crate::SingleOwnerAsyncProvider::bind)
    pub origin: Option<Type>,
    /// The scope of the provider.
    pub scope: Scope,
    /// The color of the constructor.
    pub color: Option<Color>,
    /// Whether the provider is conditional.
    pub conditional: bool,
}

impl Definition {
    pub(crate) fn new<T: 'static>(
        name: Cow<'static, str>,
        scope: Scope,
        color: Option<Color>,
        conditional: bool,
    ) -> Self {
        Self {
            key: Key::new::<T>(name),
            origin: None,
            scope,
            color,
            conditional,
        }
    }

    pub(crate) fn bind<T: 'static>(self) -> Definition {
        let Definition {
            key: Key { name, ty },
            scope,
            color,
            conditional,
            origin: _origin,
        } = self;

        Self {
            key: Key::new::<T>(name),
            origin: Some(ty),
            scope,
            color,
            conditional,
        }
    }
}

/// Represents the scope of the provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// singleton scope.
    ///
    /// 1. the constructor run only once.
    /// 2. the type implements [`Clone`] trait.
    /// 3. instances taken from context can be either instances with ownership or reference instances.
    Singleton,
    /// transient scope.
    ///
    /// 1. the constructor run every time.
    /// 2. instances taken from the context are instances with ownership.
    Transient,
    /// single owner scope.
    ///
    /// 1. the constructor run only once.
    /// 2. instances taken from the context are reference instances.
    SingleOwner,
}

/// Represents the color of the function, i.e., async or sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    /// async function
    Async,
    /// sync function
    Sync,
}

/// A trait for giving a type a default [`Provider`].
///
/// Define this trait so that the purpose is not to be implemented manually,
/// but to use the [`#[Singleton]`](crate::Singleton), [`#[Transient]`](crate::Transient) or [`#[SingleOwner]`](crate::SingleOwner) attribute macros to generate the implementation.
///
/// # Example
///
/// ```rust
/// use rudi::{DefaultProvider, Provider, Singleton, Transient};
///
/// #[Transient]
/// struct A;
///
/// #[Singleton]
/// fn Number() -> i32 {
///     42
/// }
///
/// fn main() {
///     let _: Provider<A> = <A as DefaultProvider>::provider();
///     let _: Provider<i32> = <Number as DefaultProvider>::provider();
/// }
/// ```
pub trait DefaultProvider {
    /// The generic of the [`Provider`].
    type Type: Send + Sync + 'static;

    /// Returns a default [`Provider`] for the implementation.
    fn provider() -> Provider<Self::Type>;
}

pub(crate) enum Constructor<T: Send + Sync> {
    Async(Arc<dyn for<'a> Fn(&'a mut ApplicationContext) -> BoxFuture<'a, T> + Send + Sync>),
    Sync(Arc<dyn Fn(&mut ApplicationContext) -> T + Send + Sync>),
}

impl<T: Send + Sync> Clone for Constructor<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Async(c) => Self::Async(Arc::clone(c)),
            Self::Sync(c) => Self::Sync(Arc::clone(c)),
        }
    }
}

/// Represents the eager create function.
#[derive(Clone)]
pub enum EagerCreateFunction {
    /// async eager create function.
    Async(fn(&mut ApplicationContext, Cow<'static, str>) -> BoxFuture<'static, ()>),
    /// sync eager create function.
    Sync(fn(&mut ApplicationContext, Cow<'static, str>)),
    /// no eager create function.
    None,
}

/// Represents the provider of an instance of type `T`.
///
/// This struct is just a generic, intermediate representation of `Provider`,
/// there is no pub method to direct create this struct,
/// Please use the following functions or attribute macros to create the various `Provider` types that implement `Into<Provider>`:
/// - functions
///   - [`singleton`](crate::singleton)
///   - [`transient`](crate::transient)
///   - [`single_owner`](crate::single_owner)
///   - [`singleton_async`](crate::singleton_async)
///   - [`transient_async`](crate::transient_async)
///   - [`single_owner_async`](crate::single_owner_async)
/// - attribute macros
///   - [`Singleton`](crate::Singleton)
///   - [`Transient`](crate::Transient)
///   - [`SingleOwner`](crate::SingleOwner)
#[derive(Clone)]
pub struct Provider<T: Send + Sync> {
    definition: Definition,
    eager_create: bool,
    condition: Option<fn(&ApplicationContext) -> bool>,
    constructor: Constructor<T>,
    clone_instance: Option<fn(&T) -> T>,
    eager_create_function: EagerCreateFunction,
    binding_providers: Option<Vec<DynProvider>>,
    binding_definitions: Option<Vec<Definition>>,
}

impl<T: Send + Sync> Provider<T> {
    /// Returns the [`Definition`] of the provider.
    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    /// Returns whether the provider is eager create.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns definitions of the binding providers.
    pub fn binding_definitions(&self) -> Option<&Vec<Definition>> {
        self.binding_definitions.as_ref()
    }

    /// Returns an option of the condition function.
    pub fn condition(&self) -> Option<fn(&ApplicationContext) -> bool> {
        self.condition
    }

    pub(crate) fn constructor(&self) -> Constructor<T> {
        self.constructor.clone()
    }

    pub(crate) fn clone_instance(&self) -> Option<fn(&T) -> T> {
        self.clone_instance
    }
}

impl<T: 'static + Send + Sync> Provider<T> {
    pub(crate) fn with_name(
        name: Cow<'static, str>,
        scope: Scope,
        eager_create: bool,
        condition: Option<fn(&ApplicationContext) -> bool>,
        constructor: Constructor<T>,
        clone_instance: Option<fn(&T) -> T>,
        eager_create_function: EagerCreateFunction,
    ) -> Self {
        let definition = Definition::new::<T>(
            name,
            scope,
            Some(match constructor {
                Constructor::Async(_) => Color::Async,
                Constructor::Sync(_) => Color::Sync,
            }),
            condition.is_some(),
        );

        Provider {
            definition,
            eager_create,
            condition,
            constructor,
            clone_instance,
            eager_create_function,
            binding_providers: None,
            binding_definitions: None,
        }
    }

    pub(crate) fn with_definition(
        definition: Definition,
        eager_create: bool,
        condition: Option<fn(&ApplicationContext) -> bool>,
        constructor: Constructor<T>,
        clone_instance: Option<fn(&T) -> T>,
        eager_create_function: EagerCreateFunction,
    ) -> Self {
        Provider {
            definition,
            eager_create,
            condition,
            constructor,
            clone_instance,
            eager_create_function,
            binding_providers: None,
            binding_definitions: None,
        }
    }

    pub(crate) fn never_construct(name: Cow<'static, str>, scope: Scope) -> Self {
        Provider {
            definition: Definition::new::<T>(name, scope, None, false),
            eager_create: false,
            condition: None,
            constructor: Constructor::Sync(Arc::new(|_| panic!("never construct"))),
            clone_instance: None,
            eager_create_function: EagerCreateFunction::None,
            binding_providers: None,
            binding_definitions: None,
        }
    }
}

/// Represents a [`Provider`] that erased its type.

pub struct DynProvider {
    definition: Definition,
    eager_create: bool,
    condition: Option<fn(&ApplicationContext) -> bool>,
    eager_create_function: EagerCreateFunction,
    binding_providers: Option<Vec<DynProvider>>,
    binding_definitions: Option<Vec<Definition>>,
    origin: Arc<dyn Any + Send + Sync>,
}

impl Clone for DynProvider {
    fn clone(&self) -> Self {
        Self {
            definition: self.definition.clone(),
            eager_create: self.eager_create.clone(),
            condition: self.condition.clone(),
            eager_create_function: self.eager_create_function.clone(),
            binding_providers: self.binding_providers.clone(),
            binding_definitions: self.binding_definitions.clone(),
            origin: Arc::clone(&self.origin),
        }
    }
}

impl DynProvider {
    /// Returns the [`Definition`] of the provider.
    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    /// Returns whether the provider is eager create.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns definitions of the binding providers.
    pub fn binding_definitions(&self) -> Option<&Vec<Definition>> {
        self.binding_definitions.as_ref()
    }

    /// Returns a reference of the origin [`Provider`].
    pub fn as_provider<T: 'static + Send + Sync>(&self) -> Option<&Provider<T>> {
        self.origin.downcast_ref::<Provider<T>>()
    }

    /// Returns an option of the condition function.
    pub fn condition(&self) -> Option<fn(&ApplicationContext) -> bool> {
        self.condition
    }

    pub(crate) fn key(&self) -> &Key {
        &self.definition.key
    }

    pub(crate) fn eager_create_function(&self) -> EagerCreateFunction {
        self.eager_create_function.clone()
    }

    pub(crate) fn binding_providers(&mut self) -> Option<Vec<DynProvider>> {
        self.binding_providers.take()
    }
}

impl<T: 'static + Send + Sync> From<Provider<T>> for DynProvider {
    fn from(mut value: Provider<T>) -> Self {
        Self {
            definition: value.definition.clone(),
            eager_create: value.eager_create,
            condition: value.condition,
            eager_create_function: value.eager_create_function.clone(),
            binding_providers: value.binding_providers.take(),
            binding_definitions: value.binding_definitions.clone(),
            origin: Arc::new(value),
        }
    }
}

fn sync_constructor<T, U, F>(
    name: Cow<'static, str>,
    transform: F,
) -> Arc<dyn Fn(&mut ApplicationContext) -> U + Send + Sync>
where
    T: 'static + Send + Sync,
    F: Fn(T) -> U + 'static + Send + Sync,
    U: Send + Sync,
{
    let constructor = move |cx: &mut ApplicationContext| {
        let instance = cx.resolve_with_name(name.clone());
        transform(instance)
    };

    Arc::new(constructor)
}

fn sync_eager_create_function<T: 'static + Send + Sync>() -> fn(&mut ApplicationContext, Cow<'static, str>) {
    |cx, name| {
        cx.just_create::<T>(name);
    }
}

fn create_async<T: 'static + Send + Sync>(
    _cx: &mut ApplicationContext,
    name: Cow<'static, str>,
) -> BoxFuture<'static, ()> {
    let name = name.clone();
    Box::pin(async move {
        let mut temp_cx = ApplicationContext::default();
        temp_cx.just_create_async::<T>(name).await;
    })
}

fn async_eager_create_function<T: 'static + Send + Sync>(
) -> fn(&mut ApplicationContext, Cow<'static, str>) -> BoxFuture<'static, ()> {
    create_async::<T>
}

macro_rules! define_provider_common {
    (
        $provider:ident,
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        /// Represents a specialized [`Provider`].
        ///
        #[doc = concat!("Use the [`", stringify!($function), "`] function to create this provider.")]
        pub struct $provider<T: Send + Sync> {
            constructor: Constructor<T>,
            name: Cow<'static, str>,
            eager_create: bool,
            condition: Option<fn(&ApplicationContext) -> bool>,
            bind_closures: Vec<Box<dyn FnOnce(Definition, bool, Option<fn(&ApplicationContext) -> bool>) -> DynProvider>>,
        }

        impl<T: Send + Sync> $provider<T> {
            /// Sets the name of the provider.
            pub fn name<N>(mut self, name: N) -> Self
            where
                N: Into<Cow<'static, str>>,
            {
                self.name = name.into();
                self
            }

            /// Sets whether the provider is eager to create.
            pub fn eager_create(mut self, eager_create: bool) -> Self {
                self.eager_create = eager_create;
                self
            }

            /// Sets whether or not to insert the provider into the [`ApplicationContext`] based on the condition.
            pub fn condition(mut self, condition: Option<fn(&ApplicationContext) -> bool>) -> Self {
                self.condition = condition;
                self
            }
        }

        impl<T: 'static + Send + Sync $(+ $bound)*> From<$provider<T>> for DynProvider {
            fn from(value: $provider<T>) -> Self {
                DynProvider::from(Provider::from(value))
            }
        }
    };
}

macro_rules! define_provider_sync {
    (
        $provider:ident,
        $scope:expr,
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        #[doc = concat!("create a [`", stringify!($provider), "`] instance")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use rudi::{", stringify!($function), ", ", stringify!($provider), "};")]
        ///
        /// #[derive(Clone)]
        /// struct A(i32);
        ///
        /// fn main() {
        #[doc = concat!("    let _: ", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()));")]
        /// }
        /// ```
        pub fn $function<T: Send + Sync, C>(constructor: C) -> $provider<T>
        where
            C: Fn(&mut ApplicationContext) -> T + 'static + Send + Sync,
        {
            $provider {
                constructor: Constructor::Sync(Arc::new(constructor)),
                name: Cow::Borrowed(""),
                eager_create: false,
                condition: None,
                bind_closures: Vec::new(),
            }
        }

        impl<T: 'static + Send + Sync> $provider<T> {
            /// Create a provider of type [`Provider<U>`], save it to the current provider.
            ///
            /// This method accepts a parameter of `fn(T) -> U`, which in combination
            /// with the current provider's constructor of type `fn(&mut ApplicationContext) -> T`,
            /// creates a `Provider<U>` with constructor `fn(&mut ApplicationContext) -> U`
            /// and other fields consistent with the current provider.
            ///
            /// All bound providers will be registered together
            /// when the current provider is registered in the [`ApplicationContext`].
            ///
            /// # Example
            ///
            /// ```rust
            /// use std::{fmt::Debug, rc::Arc, sync::Arc};
            ///
            #[doc = concat!("use rudi::{", stringify!($function), ", Provider, ", stringify!($provider), "};")]
            ///
            /// #[derive(Clone, Debug)]
            /// struct A(i32);
            ///
            /// fn into_debug(a: A) -> Arc<dyn Debug> {
            ///     Arc::new(a)
            /// }
            ///
            /// fn main() {
            #[doc = concat!("    let p: ", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()))")]
            ///         .bind(Arc::new)
            ///         .bind(Arc::new)
            ///         .bind(Box::new)
            ///         .bind(into_debug);
            ///
            ///     let p: Provider<A> = p.into();
            ///
            ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
            /// }
            /// ```
            pub fn bind<U, F>(mut self, transform: F) -> Self
            where
                U: 'static + Send + Sync $(+ $bound)*,
                F: Fn(T) -> U + 'static + Send + Sync,
            {
                let bind_closure = |definition: Definition, eager_create: bool, condition: Option<fn(&ApplicationContext) -> bool>| {
                    let name = definition.key.name.clone();

                    Provider::with_definition(
                        definition.bind::<U>(),
                        eager_create,
                        condition,
                        Constructor::Sync(sync_constructor(name, transform)),
                        $clone_instance,
                        EagerCreateFunction::Sync(
                            sync_eager_create_function::<U>()
                        ),
                    )
                    .into()
                };

                let bind_closure = Box::new(bind_closure);
                self.bind_closures.push(bind_closure);

                self
            }
        }

        impl<T: 'static + Send + Sync $(+ $bound)*> From<$provider<T>> for Provider<T> {
            fn from(value: $provider<T>) -> Self {
                let $provider {
                    constructor,
                    name,
                    eager_create,
                    condition,
                    bind_closures,
                } = value;

                let mut provider = Provider::with_name(
                    name,
                    $scope,
                    eager_create,
                    condition,
                    constructor,
                    $clone_instance,
                    EagerCreateFunction::Sync(
                        sync_eager_create_function::<T>()
                    ),
                );

                if bind_closures.is_empty() {
                    return provider;
                }

                let definition = &provider.definition;

                let (definitions, providers) = bind_closures.into_iter()
                    .map(|bind_closure| {
                        let provider = bind_closure(definition.clone(), eager_create, condition);
                        (provider.definition.clone(), provider)
                    })
                    .unzip();

                provider.binding_definitions = Some(definitions);
                provider.binding_providers = Some(providers);

                provider
            }
        }
    };
}

macro_rules! define_provider_async {
    (
        $provider:ident,
        $scope:expr,
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        #[doc = concat!("Create a [`", stringify!($provider), "`] instance")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt, ", stringify!($provider), "};")]
        ///
        /// #[derive(Clone)]
        /// struct A(i32);
        ///
        /// fn main() {
        #[doc = concat!("    let _: ", stringify!($provider), "<A> =")]
        #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed());")]
        /// }
        /// ```
        pub fn $function<T: Send + Sync + 'static, C>(constructor: C) -> $provider<T>
        where
            C: Fn(&mut ApplicationContext) -> BoxFuture<'static, T> + 'static + Send + Sync,
        {
            $provider {
                constructor: Constructor::Async(Arc::new(move |cx| {
                    let fut = constructor(cx);
                    Box::pin(async move { fut.await }) as BoxFuture<'_, T>
                })),
                name: Cow::Borrowed(""),
                eager_create: false,
                condition: None,
                bind_closures: Vec::new(),
            }
        }

        impl<T: 'static + Send + Sync> $provider<T> {
            /// Create a provider of type [`Provider<U>`], save it to the current provider.
            ///
            /// This method accepts a parameter of `fn(T) -> U`, which in combination
            /// with the current provider's constructor of type `async fn(&mut ApplicationContext) -> T`,
            /// creates a `Provider<U>` with constructor `async fn(&mut ApplicationContext) -> U`
            /// and other fields consistent with the current provider.
            ///
            /// All bound providers will be registered together
            /// when the current provider is registered in the [`ApplicationContext`].
            ///
            /// # Example
            ///
            /// ```rust
            /// use std::{fmt::Debug, rc::Arc, sync::Arc};
            ///
            #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt, Provider, ", stringify!($provider), "};")]
            ///
            /// #[derive(Clone, Debug)]
            /// struct A(i32);
            ///
            /// fn into_debug(a: A) -> Arc<dyn Debug> {
            ///     Arc::new(a)
            /// }
            ///
            /// fn main() {
            #[doc = concat!("    let p: ", stringify!($provider), "<A> =")]
            #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed())")]
            ///             .bind(Arc::new)
            ///             .bind(Arc::new)
            ///             .bind(Box::new)
            ///             .bind(into_debug);
            ///
            ///     let p: Provider<A> = p.into();
            ///
            ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
            /// }
            /// ```
            pub fn bind<U, F>(mut self, transform: F) -> Self
            where
                U: 'static + Send + Sync $(+ $bound)*,
                F: Fn(T) -> U + 'static + Clone + Send + Sync,
            {
                let bind_closure = |definition: Definition, eager_create: bool, condition: Option<fn(&ApplicationContext) -> bool>| {
                    let name = definition.key.name.clone();

                    Provider::with_definition(
                        definition.bind::<U>(),
                        eager_create,
                        condition,
                        Constructor::Async(Arc::new(move |cx| {
                            let instance = cx.resolve_with_name(name.clone());
                            let transform_clone = transform.clone();
                            Box::pin(async move { transform_clone(instance) }) as BoxFuture<'_, U>
                        })),
                        $clone_instance,
                        EagerCreateFunction::Async(
                            async_eager_create_function::<U>()
                        ),
                    )
                    .into()
                };

                let bind_closure = Box::new(bind_closure);
                self.bind_closures.push(bind_closure);

                self
            }
        }

        impl<T: 'static + Send + Sync $(+ $bound)*> From<$provider<T>> for Provider<T> {
            fn from(value: $provider<T>) -> Self {
                let $provider {
                    constructor,
                    name,
                    eager_create,
                    condition,
                    bind_closures,
                } = value;

                let mut provider = Provider::with_name(
                    name,
                    $scope,
                    eager_create,
                    condition,
                    constructor,
                    $clone_instance,
                    EagerCreateFunction::Async(
                        async_eager_create_function::<T>()
                    ),
                );

                if bind_closures.is_empty() {
                    return provider;
                }

                let definition = &provider.definition;

                let (definitions, providers) = bind_closures.into_iter()
                    .map(|bind_closure| {
                        let provider = bind_closure(definition.clone(), eager_create, condition);
                        (provider.definition.clone(), provider)
                    })
                    .unzip();

                provider.binding_definitions = Some(definitions);
                provider.binding_providers = Some(providers);

                provider
            }
        }
    };
}

define_provider_common!(SingletonProvider, singleton, Some(Clone::clone), + Clone);
define_provider_common!(TransientProvider, transient, None,);
define_provider_common!(SingleOwnerProvider, single_owner, None,);
define_provider_common!(SingletonAsyncProvider, singleton_async, Some(Clone::clone), + Clone);
define_provider_common!(TransientAsyncProvider, transient_async, None,);
define_provider_common!(SingleOwnerAsyncProvider, single_owner_async, None,);

define_provider_sync!(SingletonProvider, Scope::Singleton, singleton, Some(Clone::clone), + Clone);
define_provider_sync!(TransientProvider, Scope::Transient, transient, None,);
define_provider_sync!(SingleOwnerProvider, Scope::SingleOwner, single_owner, None,);

define_provider_async!(SingletonAsyncProvider, Scope::Singleton, singleton_async, Some(Clone::clone), + Clone);
define_provider_async!(
    TransientAsyncProvider,
    Scope::Transient,
    transient_async,
    None,
);
define_provider_async!(
    SingleOwnerAsyncProvider,
    Scope::SingleOwner,
    single_owner_async,
    None,
);

/// Represents a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance.
pub struct Single<T> {
    instance: T,
    clone: Option<fn(&T) -> T>,
}

impl<T> Single<T> {
    pub(crate) fn new(instance: T, clone: Option<fn(&T) -> T>) -> Self {
        Self { instance, clone }
    }

    /// Returns the owned instance.
    pub fn get_owned(&self) -> Option<T> {
        self.clone.map(|clone| clone(&self.instance))
    }

    /// Returns a reference to the instance.
    pub fn get_ref(&self) -> &T {
        &self.instance
    }
}

/// Represents a [`Single`] that erased its type.

pub struct DynSingle {
    origin: Arc<dyn Any + Send + Sync>,
}

impl Clone for DynSingle {
    fn clone(&self) -> Self {
        Self {
            origin: Arc::new(self.origin.clone()),
        }
    }
}

impl DynSingle {
    /// Returns a reference of the origin [`Single`].
    pub fn as_single<T: 'static>(&self) -> Option<&Single<T>> {
        self.origin.downcast_ref::<Single<T>>()
    }
}

impl<T: 'static + Send + Sync> From<Single<T>> for DynSingle {
    fn from(value: Single<T>) -> Self {
        Self {
            origin: Arc::new(value),
        }
    }
}

