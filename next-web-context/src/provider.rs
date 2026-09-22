//! The dependency injection service provider interface.
//!
//! A [`Provider`] describes how an instance is created, which name it is
//! registered under and when it has to exist. Providers are what the attribute
//! macros of the framework produce, and they are the values an application
//! context resolves its singletons from.
//!
//! The constructors of a provider receive `&mut dyn ApplicationContext`, so the
//! providers of a library do not depend on the concrete context of the
//! application. The generic methods used inside a constructor (for example
//! `resolve_with_name`) are provided by
//! [`ApplicationContextExt`](crate::ApplicationContextExt).

use std::any::Any;
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use next_web_singletons::factory::support::{Key, Type};

use crate::{ApplicationContext, ApplicationContextExt};
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

    pub fn submodules(&mut self) -> Option<Vec<ResolveModule>> {
        self.submodules.take()
    }

    pub fn providers(self) -> Vec<DynProvider> {
        self.providers
    }
}

/// Represents a unique key for a provider.
#[derive(Clone, Debug)]

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
    pub fn new<T: 'static>(
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

    pub fn bind<T: 'static>(self) -> Definition {
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
/// use next-web-context::{DefaultProvider, Provider, Singleton, Transient};
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

#[doc(hidden)]
pub enum Constructor<T: Send + Sync> {
    Async(Arc<dyn for<'a> Fn(&'a mut dyn ApplicationContext) -> BoxFuture<'a, T> + Send + Sync>),
    Sync(Arc<dyn Fn(&mut dyn ApplicationContext) -> T + Send + Sync>),
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
    Async(fn(&mut dyn ApplicationContext, Cow<'static, str>) -> BoxFuture<'static, ()>),
    /// sync eager create function.
    Sync(fn(&mut dyn ApplicationContext, Cow<'static, str>)),
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
    condition: Option<fn(&dyn ApplicationContext) -> bool>,
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
    pub fn condition(&self) -> Option<fn(&dyn ApplicationContext) -> bool> {
        self.condition
    }

    pub fn constructor(&self) -> Constructor<T> {
        self.constructor.clone()
    }

    pub fn clone_instance(&self) -> Option<fn(&T) -> T> {
        self.clone_instance
    }
}

impl<T: 'static + Send + Sync> Provider<T> {
    pub fn with_name(
        name: Cow<'static, str>,
        scope: Scope,
        eager_create: bool,
        condition: Option<fn(&dyn ApplicationContext) -> bool>,
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

    pub fn with_definition(
        definition: Definition,
        eager_create: bool,
        condition: Option<fn(&dyn ApplicationContext) -> bool>,
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

    pub fn never_construct(name: Cow<'static, str>, scope: Scope) -> Self {
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
    condition: Option<fn(&dyn ApplicationContext) -> bool>,
    eager_create_function: EagerCreateFunction,
    binding_providers: Option<Vec<DynProvider>>,
    binding_definitions: Option<Vec<Definition>>,
    origin: Arc<dyn Any + Send + Sync>,
    /// Creates the instance of the origin [`Provider`], with its type erased.
    build: fn(&(dyn Any + Send + Sync), &mut dyn ApplicationContext) -> Box<dyn Any + Send + Sync>,
    /// Produces an owned copy of an instance created by the origin provider.
    ///
    /// The function receives the origin provider and the instance, so that it
    /// does not have to capture them.
    clone_instance: Option<
        fn(&(dyn Any + Send + Sync), &(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync>,
    >,
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
            build: self.build,
            clone_instance: self.clone_instance,
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
    pub fn condition(&self) -> Option<fn(&dyn ApplicationContext) -> bool> {
        self.condition
    }

    pub fn key(&self) -> &Key {
        &self.definition.key
    }

    pub fn eager_create_function(&self) -> EagerCreateFunction {
        self.eager_create_function.clone()
    }

    pub fn binding_providers(&mut self) -> Option<Vec<DynProvider>> {
        self.binding_providers.take()
    }

    /// Returns the origin value of the provider.
    ///
    /// A context stores the origin together with the clone function of an
    /// instance, so that it can produce owned copies of the instance later.
    pub fn origin(&self) -> Arc<dyn Any + Send + Sync> {
        Arc::clone(&self.origin)
    }

    /// Creates the instance of the provider and returns it with its type erased.
    ///
    /// # Panics
    ///
    /// Panics when the provider has an asynchronous constructor, because a
    /// context resolves its providers synchronously.
    ///
    /// # Arguments
    ///
    /// * `cx` - The context used to resolve the dependencies of the instance.
    pub fn build_boxed(&self, cx: &mut dyn ApplicationContext) -> Box<dyn Any + Send + Sync> {
        (self.build)(self.origin.as_ref(), cx)
    }

    /// Returns the function producing owned copies of an instance of the
    /// provider, when the instance may be handed out as an owned value.
    ///
    /// The returned function expects the origin of the provider (see
    /// [`Self::origin`]) and an instance created by it.
    pub fn clone_instance(
        &self,
    ) -> Option<fn(&(dyn Any + Send + Sync), &(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync>>
    {
        self.clone_instance
    }
}

impl<T: 'static + Send + Sync> From<Provider<T>> for DynProvider {
    fn from(mut value: Provider<T>) -> Self {
        /// Creates the instance of the origin provider.
        fn build<T: 'static + Send + Sync>(
            origin: &(dyn Any + Send + Sync),
            cx: &mut dyn ApplicationContext,
        ) -> Box<dyn Any + Send + Sync> {
            let provider = origin
                .downcast_ref::<Provider<T>>()
                .expect("the erased provider holds its own type");

            match provider.constructor() {
                Constructor::Sync(constructor) => Box::new(constructor(cx)),
                Constructor::Async(_) => panic!(
                    "an asynchronous provider is not supported by the context yet: {:?}",
                    provider.definition()
                ),
            }
        }

        /// Copies an instance created by the origin provider.
        fn clone_instance<T: 'static + Send + Sync>(
            origin: &(dyn Any + Send + Sync),
            instance: &(dyn Any + Send + Sync),
        ) -> Box<dyn Any + Send + Sync> {
            let provider = origin
                .downcast_ref::<Provider<T>>()
                .expect("the erased provider holds its own type");
            let clone = provider
                .clone_instance()
                .expect("the provider records a clone function");
            let instance = instance
                .downcast_ref::<T>()
                .expect("the instance was created by its provider");

            Box::new(clone(instance))
        }

        let clone_instance = value.clone_instance.is_some().then_some(
            clone_instance::<T>
                as fn(
                    &(dyn Any + Send + Sync),
                    &(dyn Any + Send + Sync),
                ) -> Box<dyn Any + Send + Sync>,
        );

        Self {
            definition: value.definition.clone(),
            eager_create: value.eager_create,
            condition: value.condition,
            eager_create_function: value.eager_create_function.clone(),
            binding_providers: value.binding_providers.take(),
            binding_definitions: value.binding_definitions.clone(),
            origin: Arc::new(value),
            build: build::<T>,
            clone_instance,
        }
    }
}

#[derive(Clone)]
pub struct BoxValue<T>(pub T);

fn sync_constructor<T, U, F>(
    name: Cow<'static, str>,
    transform: F,
) -> Arc<dyn Fn(&mut dyn ApplicationContext) -> U + Send + Sync>
where
    T: 'static + Send + Sync,
    F: Fn(T) -> U + 'static + Send + Sync,
    U: Send + Sync,
{
    let constructor = move |cx: &mut dyn ApplicationContext| {
        let instance = cx.resolve_with_name(name.clone());
        transform(instance)
    };

    Arc::new(constructor)
}

fn sync_eager_create_function<T: 'static + Send + Sync>()
-> fn(&mut dyn ApplicationContext, Cow<'static, str>) {
    |cx, name| {
        // Resolving the instance is enough: a singleton is cached by the
        // context, so the instance that is resolved later is the one created
        // here.
        let _ = crate::ApplicationContextExt::resolve_option_with_name::<T>(cx, name);
    }
}

fn create_async<T: 'static + Send + Sync>(
    _cx: &mut dyn ApplicationContext,
    name: Cow<'static, str>,
) -> BoxFuture<'static, ()> {
    // Asynchronous eager creation is not supported yet: the instance is created
    // when it is resolved for the first time instead.
    let _ = name;
    let _ = std::marker::PhantomData::<fn() -> T>;
    Box::pin(async {})
}

fn async_eager_create_function<T: 'static + Send + Sync>()
-> fn(&mut dyn ApplicationContext, Cow<'static, str>) -> BoxFuture<'static, ()> {
    create_async::<T>
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Type used by the provider tests.
    #[derive(Clone)]
    struct Service;

    #[test]
    fn builds_a_named_singleton_provider() {
        let provider: Provider<Service> = Provider::from(singleton(|_| Service).name("service"));

        assert_eq!(provider.definition().key.name, "service");
        assert_eq!(provider.definition().scope, Scope::Singleton);

        let provider: DynProvider = provider.into();
        assert_eq!(provider.definition().key.name, "service");
    }

    #[test]
    fn binds_a_provider_to_another_type() {
        let provider: Provider<Service> =
            Provider::from(singleton(|_| Service).bind(|_service| String::from("bound")));

        let definitions = provider
            .binding_definitions()
            .expect("the bound definition is recorded");

        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].key.name, "");
        assert_eq!(definitions[0].key.ty, Type::new::<String>());
    }

    #[test]
    fn a_module_collects_its_providers() {
        struct Services;

        impl Module for Services {
            fn providers() -> Vec<DynProvider> {
                vec![singleton(|_| Service).name("service").into()]
            }
        }

        let module = ResolveModule::new::<Services>();
        assert_eq!(module.ty(), Type::new::<Services>());
        assert_eq!(module.providers().len(), 1);
    }
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
            condition: Option<fn(&dyn ApplicationContext) -> bool>,
            bind_closures: Vec<Box<dyn FnOnce(Definition, bool, Option<fn(&dyn ApplicationContext) -> bool>) -> DynProvider>>,
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
            pub fn condition(mut self, condition: Option<fn(&dyn ApplicationContext) -> bool>) -> Self {
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
        #[doc = concat!("use next-web-context::{", stringify!($function), ", ", stringify!($provider), "};")]
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
            C: Fn(&mut dyn ApplicationContext) -> T + 'static + Send + Sync,
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
            /// with the current provider's constructor of type `fn(&mut dyn ApplicationContext) -> T`,
            /// creates a `Provider<U>` with constructor `fn(&mut dyn ApplicationContext) -> U`
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
            #[doc = concat!("use next-web-context::{", stringify!($function), ", Provider, ", stringify!($provider), "};")]
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
                let bind_closure = |definition: Definition, eager_create: bool, condition: Option<fn(&dyn ApplicationContext) -> bool>| {
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
        #[doc = concat!("use next-web-context::{", stringify!($function), ", FutureExt, ", stringify!($provider), "};")]
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
            C: Fn(&mut dyn ApplicationContext) -> BoxFuture<'static, T> + 'static + Send + Sync,
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
            /// with the current provider's constructor of type `async fn(&mut dyn ApplicationContext) -> T`,
            /// creates a `Provider<U>` with constructor `async fn(&mut dyn ApplicationContext) -> U`
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
            #[doc = concat!("use next-web-context::{", stringify!($function), ", FutureExt, Provider, ", stringify!($provider), "};")]
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
                let bind_closure = |definition: Definition, eager_create: bool, condition: Option<fn(&dyn ApplicationContext) -> bool>| {
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
