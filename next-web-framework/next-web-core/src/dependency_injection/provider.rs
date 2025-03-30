// use std::{any::Any, borrow::Cow, sync::Arc};

// use crate::context::application_context::ApplicationContext;

// use super::{color::Color, definition::{Definition, Key}, scope::Scope};
// use super::future::{FutureExt, BoxFuture};


// pub struct Provider<T: Send + Sync> {
//     definition: Definition,
//     order: Option<i32>,
//     constructor: Constructor<T>,
//     clone_instance: Option<fn(&T) -> T>,
//     condition: Option<fn(&ApplicationContext) -> bool>,
//     binding_providers: Option<Vec<DynProvider>>,
//     binding_definitions: Option<Vec<Definition>>,
// }

// pub trait DefaultProvider {
//     /// The generic of the [`Provider`].
//     type Type: Send + Sync;

//     /// Returns a default [`Provider`] for the implementation.
//     fn provider() -> Provider<Self::Type>;
// }

// pub(crate) enum Constructor<T> {
//     Sync(Arc<dyn Fn(&mut ApplicationContext) -> T +  Send + Sync>),
//     #[allow(clippy::type_complexity)]
//     Async(Arc<dyn for<'a>  Fn(&'a mut ApplicationContext) -> BoxFuture<'a, T> +  Send + Sync>),
//     None,
// }

// impl<T: Send + Sync> Clone for Constructor<T> {
//     fn clone(&self) -> Self {
//         match self {
//             Self::Async(c) => Self::Async(Arc::clone(c)),
//             Self::Sync(c) => Self::Sync(Arc::clone(c)),
//             Self::None => Self::None,
//         }
//     }
// }


// impl<T: Send + Sync> Provider<T> {
//     /// Returns the [`Definition`] of the provider.
//     pub fn definition(&self) -> &Definition {
//         &self.definition
//     }

//     /// Returns whether the provider is eager create.
//     pub fn order(&self) -> Option<i32> {
//         self.order
//     }

//     /// Returns definitions of the binding providers.
//     pub fn binding_definitions(&self) -> Option<&Vec<Definition>> {
//         self.binding_definitions.as_ref()
//     }

//     /// Returns an option of the condition function.
//     pub fn condition(&self) -> Option<fn(&ApplicationContext) -> bool> {
//         self.condition
//     }

//     pub(crate) fn constructor(&self) -> Constructor<T> {
//         self.constructor.clone()
//     }

//     pub(crate) fn clone_instance(&self) -> Option<fn(&T) -> T> {
//         self.clone_instance
//     }
// }


// impl<T: 'static + Send + Sync> Provider<T> {
//     pub(crate) fn with_name(
//         name: Cow<'static, str>,
//         scope: Scope,
//         order: Option<i32>,
//         condition: Option<fn(&ApplicationContext) -> bool>,
//         constructor: Constructor<T>,
//         clone_instance: Option<fn(&T) -> T>,
//     ) -> Self {
//         let definition = Definition::new::<T>(
//             name,
//             scope,
//             Some(match constructor {
//                 Constructor::Async(_) => Color::Async,
//                 Constructor::Sync(_) => Color::Sync,
//                 Constructor::None => unreachable!(),
//             }),
//             condition.is_some(),
//         );

//         Provider {
//             definition,
//             order,
//             condition,
//             constructor,
//             clone_instance,
//             binding_providers: None,
//             binding_definitions: None,
//         }
//     }

//     pub(crate) fn with_definition(
//         definition: Definition,
//         order: Option<i32>,
//         condition: Option<fn(&ApplicationContext) -> bool>,
//         constructor: Constructor<T>,
//         clone_instance: Option<fn(&T) -> T>,
//     ) -> Self {
//         Provider {
//             definition,
//             order,
//             condition,
//             constructor,
//             clone_instance,
//             binding_providers: None,
//             binding_definitions: None,
//         }
//     }

//     pub(crate) fn never_construct(name: Cow<'static, str>, scope: Scope) -> Self {
//         Provider {
//             definition: Definition::new::<T>(name, scope, None, false),
//             order: None,
//             condition: None,
//             constructor: Constructor::None,
//             clone_instance: None,
//             binding_providers: None,
//             binding_definitions: None,
//         }
//     }
// }



// /// Represents a [`Provider`] that erased its type.

// pub struct DynProvider {
//     definition: Definition,
//     order: Option<i32>,
//     condition: Option<fn(&ApplicationContext) -> bool>,
//     binding_providers: Option<Vec<DynProvider>>,
//     binding_definitions: Option<Vec<Definition>>,
//     origin: Arc<dyn Any + Send + Sync>,
// }

// impl Clone for DynProvider {
//     fn clone(&self) -> Self {
//         Self {
//             definition: self.definition.clone(),
//             order: self.order.clone(),
//             condition: self.condition.clone(),
//             binding_providers: self.binding_providers.clone(),
//             binding_definitions: self.binding_definitions.clone(),
//             origin: Arc::clone(&self.origin),
//         }
//     }
// }

// impl DynProvider {
//     /// Returns the [`Definition`] of the provider.
//     pub fn definition(&self) -> &Definition {
//         &self.definition
//     }

//     /// Returns whether the provider is eager create.
//     pub fn order(&self) -> Option<i32> {
//         self.order
//     }

//     /// Returns definitions of the binding providers.
//     pub fn binding_definitions(&self) -> Option<&Vec<Definition>> {
//         self.binding_definitions.as_ref()
//     }

//     /// Returns a reference of the origin [`Provider`].
//     pub fn as_provider<T: 'static + Send + Sync>(&self) -> Option<&Provider<T>> {
//         self.origin.downcast_ref::<Provider<T>>()
//     }

//     /// Returns an option of the condition function.
//     pub fn condition(&self) -> Option<fn(&ApplicationContext) -> bool> {
//         self.condition
//     }

//     pub(crate) fn key(&self) -> &Key {
//         &self.definition.key
//     }

//     pub(crate) fn binding_providers(&mut self) -> Option<Vec<DynProvider>> {
//         self.binding_providers.take()
//     }
// }

// impl<T: 'static + Send + Sync> From<Provider<T>> for DynProvider {
//     fn from(mut value: Provider<T>) -> Self {
//         Self {
//             definition: value.definition.clone(),
//             order: value.order,
//             condition: value.condition,
//             binding_providers: value.binding_providers.take(),
//             binding_definitions: value.binding_definitions.clone(),
//             // 将 Arc 替换为 Arc，因为 Arc 实现了 Send 和 Sync 特质，可在线程间安全传递和共享
//             origin: Arc::new(value),
//         }
//     }
// }

// fn sync_constructor<T, U, F>(name: Cow<'static, str>, transform: F) -> Arc<dyn for<'a> Fn(&'a mut ApplicationContext) -> U + Send + Sync>
// where
//     T: 'static + Send + Sync + Clone,
//     U: 'static + Send + Sync,
//     F: Fn(T) -> U + 'static + Send + Sync,
// {
//     let constructor = move |cx: &mut ApplicationContext| {
//         let instance = cx.get_single_with_name(&name).unwrap();
//         transform(instance)
//     };

//     Arc::new(constructor)
// }


// #[allow(clippy::type_complexity)]
// fn async_constructor<T, U, F>(
//     name: Cow<'static, str>,
//     transform: F,
// ) -> Arc<dyn for<'a> Fn(&'a mut ApplicationContext) -> BoxFuture<'a, U> + Send + Sync>
// where
//     T: 'static + Send + Sync + Clone,
//     U: 'static + Send + Sync,
//     F: Fn(T) -> U + 'static + Send + Sync + Clone,
// {
//     fn helper<'a, F, T, U>(
//         cx: &'a mut ApplicationContext,
//         name: Cow<'static, str>,
//         transform: F,
//     ) -> BoxFuture<'a, U>
//     where
//         T: 'static + Send + Sync + Clone,
//         F: Fn(T) -> U + 'static + Send + Sync,
//     {
//         async move {
//             let instance = cx.get_single_with_name_async(&name).await;
//             transform(instance)
//         }
//         .boxed()
//     }

//     Arc::new(move |cx| helper(cx, name.clone(), transform.clone()))
// }

// macro_rules! define_provider_common {
//     (
//         $provider:ident,
//         $function:ident,
//         $clone_instance:expr,
//         $(+ $bound:ident)*
//     ) => {
//         /// Represents a specialized [`Provider`].
//         ///
//         #[doc = concat!("Use the [`", stringify!($function), "`] function to create this provider.")]
//         pub struct $provider<T: Send + Sync > {
//             constructor: Constructor<T>,
//             name: Cow<'static, str>,
//             order: Option<i32>,
//             condition: Option<fn(&ApplicationContext) -> bool>,
//             bind_closures: Vec<Box<dyn FnOnce(Definition, Option<i32>, Option<fn(&ApplicationContext) -> bool>) -> DynProvider>>,
//         }

//         impl<T: Send + Sync > $provider<T> {
//             /// Sets the name of the provider.
//             pub fn name<N>(mut self, name: N) -> Self
//             where
//                 N: Into<Cow<'static, str>>,
//             {
//                 self.name = name.into();
//                 self
//             }

//             /// Sets whether the provider is eager to create.
//             pub fn order(mut self, order: Option<i32>) -> Self {
//                 self.order = order;
//                 self
//             }

//             /// Sets whether or not to insert the provider into the [`ApplicationContext`] based on the condition.
//             pub fn condition(mut self, condition: Option<fn(&ApplicationContext) -> bool>) -> Self {
//                 self.condition = condition;
//                 self
//             }
//         }

//         impl<T: 'static + Send + Sync $(+ $bound)*> From<$provider<T>> for DynProvider {
//             fn from(value: $provider<T>) -> Self {
//                 DynProvider::from(Provider::from(value))
//             }
//         }
//     };
// }

// macro_rules! define_provider_sync {
//     (
//         $provider:ident,
//         $scope:expr,
//         $function:ident,
//         $clone_instance:expr,
//         $(+ $bound:ident)*
//     ) => {
//         #[doc = concat!("create a [`", stringify!($provider), "`] instance")]
//         ///
//         /// # Example
//         ///
//         /// ```rust
//         #[doc = concat!("use rudi::{", stringify!($function), ", ", stringify!($provider), "};")]
//         ///
//         /// #[derive(Clone)]
//         /// struct A(i32);
//         ///
//         /// fn main() {
//         #[doc = concat!("    let _: ", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()));")]
//         /// }
//         /// ```
//         pub fn $function<T, C>(constructor: C) -> $provider<T>
//         where
//             T:  Send + Sync,
//             C: Fn(&mut ApplicationContext) -> T + 'static + Send + Sync,
//         {
//             $provider {
//                 constructor: Constructor::Sync(Arc::new(constructor)),
//                 name: Cow::Borrowed(""),
//                 order: None,
//                 condition: None,
//                 bind_closures: Vec::new(),
//             }
//         }

//         impl<T: 'static + Send + Sync + Clone> $provider<T> {
//             /// Create a provider of type [`Provider<U>`], save it to the current provider.
//             ///
//             /// This method accepts a parameter of `fn(T) -> U`, which in combination
//             /// with the current provider's constructor of type `fn(&mut ApplicationContext) -> T`,
//             /// creates a `Provider<U>` with constructor `fn(&mut ApplicationContext) -> U`
//             /// and other fields consistent with the current provider.
//             ///
//             /// All bound providers will be registered together
//             /// when the current provider is registered in the [`ApplicationContext`].
//             ///
//             /// # Example
//             ///
//             /// ```rust
//             /// use std::{fmt::Debug, Arc::Arc, sync::Arc};
//             ///
//             #[doc = concat!("use rudi::{", stringify!($function), ", Provider, ", stringify!($provider), "};")]
//             ///
//             /// #[derive(Clone, Debug)]
//             /// struct A(i32);
//             ///
//             /// fn into_debug(a: A) -> Arc<dyn Debug> {
//             ///     Arc::new(a)
//             /// }
//             ///
//             /// fn main() {
//             #[doc = concat!("    let p: ", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()))")]
//             ///         .bind(Arc::new)
//             ///         .bind(Arc::new)
//             ///         .bind(Box::new)
//             ///         .bind(into_debug);
//             ///
//             ///     let p: Provider<A> = p.into();
//             ///
//             ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
//             /// }
//             /// ```
//             pub fn bind<U, F>(mut self, transform: F) -> Self
//             where
//                 U: 'static  + Send + Sync $(+ $bound)*,
//                 F: Fn(T) -> U + 'static + Send + Sync,
//             {
//                 let bind_closure = |definition: Definition, order: Option<i32>, condition: Option<fn(&ApplicationContext) -> bool>| {
//                     let name = definition.key.name.clone();

//                     Provider::with_definition(
//                         definition.bind::<U>(),
//                         order,
//                         condition,
//                         Constructor::Sync(sync_constructor(name, transform)),
//                         $clone_instance
//                     )
//                     .into()
//                 };

//                 let bind_closure = Box::new(bind_closure);
//                 self.bind_closures.push(bind_closure);

//                 self
//             }
//         }

//         impl<T: 'static + Send + Sync $(+ $bound)*> From<$provider<T>> for Provider<T> {
//             fn from(value: $provider<T>) -> Self {
//                 let $provider {
//                     constructor,
//                     name,
//                     order,
//                     condition,
//                     bind_closures,
//                 } = value;

//                 let mut provider = Provider::with_name(
//                     name,
//                     $scope,
//                     order,
//                     condition,
//                     constructor,
//                     $clone_instance
//                 );

//                 if bind_closures.is_empty() {
//                     return provider;
//                 }

//                 let definition = &provider.definition;

//                 let (definitions, providers) = bind_closures.into_iter()
//                     .map(|bind_closure| {
//                         let provider = bind_closure(definition.clone(), order, condition);
//                         (provider.definition.clone(), provider)
//                     })
//                     .unzip();

//                 provider.binding_definitions = Some(definitions);
//                 provider.binding_providers = Some(providers);

//                 provider
//             }
//         }
//     };
// }

// macro_rules! define_provider_async {
//     (
//         $provider:ident,
//         $scope:expr,
//         $function:ident,
//         $clone_instance:expr,
//         $(+ $bound:ident)*
//     ) => {
//         #[doc = concat!("Create a [`", stringify!($provider), "`] instance")]
//         ///
//         /// # Example
//         ///
//         /// ```rust
//         #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt, ", stringify!($provider), "};")]
//         ///
//         /// #[derive(Clone)]
//         /// struct A(i32);
//         ///
//         /// fn main() {
//         #[doc = concat!("    let _: ", stringify!($provider), "<A> =")]
//         #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed());")]
//         /// }
//         /// ```
//         pub fn $function<T, C>(constructor: C) -> $provider<T>
//         where
//             T:   Send + Sync,
//             C: for<'a> Fn(&'a mut ApplicationContext) -> BoxFuture<'a, T> + 'static + Sync + Send,
//         {
//             $provider {
//             // 问题的根源在于 `C` 类型没有实现 `Send` 和 `Sync` 特质，这使得它无法在线程间安全地发送和共享。
//             // 为了解决这个问题，我们需要确保传入的 `constructor` 实现了 `Send` 和 `Sync` 特质。
//             // 修改泛型约束，强制要求 `C` 实现 `Send` 和 `Sync` 特质。
//                 constructor: Constructor::Async(Arc::new(constructor)),
//                 name: Cow::Borrowed(""),
//                 order: None,
//                 condition: None,
//                 bind_closures: Vec::new(),
//             }
//         }

//         impl<T: 'static + Send + Sync + Clone> $provider<T> {
//             /// Create a provider of type [`Provider<U>`], save it to the current provider.
//             ///
//             /// This method accepts a parameter of `fn(T) -> U`, which in combination
//             /// with the current provider's constructor of type `async fn(&mut ApplicationContext) -> T`,
//             /// creates a `Provider<U>` with constructor `async fn(&mut ApplicationContext) -> U`
//             /// and other fields consistent with the current provider.
//             ///
//             /// All bound providers will be registered together
//             /// when the current provider is registered in the [`ApplicationContext`].
//             ///
//             /// # Example
//             ///
//             /// ```rust
//             /// use std::{fmt::Debug, Arc::Arc, sync::Arc};
//             ///
//             #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt, Provider, ", stringify!($provider), "};")]
//             ///
//             /// #[derive(Clone, Debug)]
//             /// struct A(i32);
//             ///
//             /// fn into_debug(a: A) -> Arc<dyn Debug> {
//             ///     Arc::new(a)
//             /// }
//             ///
//             /// fn main() {
//             #[doc = concat!("    let p: ", stringify!($provider), "<A> =")]
//             #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed())")]
//             ///             .bind(Arc::new)
//             ///             .bind(Arc::new)
//             ///             .bind(Box::new)
//             ///             .bind(into_debug);
//             ///
//             ///     let p: Provider<A> = p.into();
//             ///
//             ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
//             /// }
//             /// ```
//             pub fn bind<U, F>(mut self, transform: F) -> Self
//             where
//                 U: 'static + Send + Sync $(+ $bound)*,
//                 F: Fn(T) -> U + 'static + Send + Sync + Clone,
//             {
//                 let bind_closure = |definition: Definition, order: Option<i32>, condition: Option<fn(&ApplicationContext) -> bool>| {
//                     let name = definition.key.name.clone();

//                     Provider::with_definition(
//                         definition.bind::<U>(),
//                         order,
//                         condition,
//                         Constructor::Async(async_constructor(name, transform)),
//                         $clone_instance,
//                     )
//                     .into()
//                 };

//                 let bind_closure = Box::new(bind_closure);
//                 self.bind_closures.push(bind_closure);

//                 self
//             }
//         }

//         impl<T: 'static + Send + Sync $(+ $bound)*> From<$provider<T>> for Provider<T> {
//             fn from(value: $provider<T>) -> Self {
//                 let $provider {
//                     constructor,
//                     name,
//                     order,
//                     condition,
//                     bind_closures,
//                 } = value;

//                 let mut provider = Provider::with_name(
//                     name,
//                     $scope,
//                     order,
//                     condition,
//                     constructor,
//                     $clone_instance
//                 );

//                 if bind_closures.is_empty() {
//                     return provider;
//                 }

//                 let definition = &provider.definition;

//                 let (definitions, providers) = bind_closures.into_iter()
//                     .map(|bind_closure| {
//                         let provider = bind_closure(definition.clone(), order, condition);
//                         (provider.definition.clone(), provider)
//                     })
//                     .unzip();

//                 provider.binding_definitions = Some(definitions);
//                 provider.binding_providers = Some(providers);

//                 provider
//             }
//         }
//     };
// }

// define_provider_common!(SingletonProvider, singleton, Some(Clone::clone), + Clone);
// define_provider_common!(TransientProvider, transient, None,);
// define_provider_common!(SingleOwnerProvider, single_owner, None,);
// define_provider_common!(SingletonAsyncProvider, singleton_async, Some(Clone::clone), + Clone);
// define_provider_common!(TransientAsyncProvider, transient_async, None,);
// define_provider_common!(SingleOwnerAsyncProvider, single_owner_async, None,);

// define_provider_sync!(SingletonProvider, Scope::Singleton, singleton, Some(Clone::clone), + Clone);
// define_provider_sync!(TransientProvider, Scope::Transient, transient, None,);
// define_provider_sync!(SingleOwnerProvider, Scope::SingleOwner, single_owner, None,);

// define_provider_async!(SingletonAsyncProvider, Scope::Singleton, singleton_async, Some(Clone::clone), + Clone);
// define_provider_async!(
//     TransientAsyncProvider,
//     Scope::Transient,
//     transient_async,
//     None,
// );
// define_provider_async!(
//     SingleOwnerAsyncProvider,
//     Scope::SingleOwner,
//     single_owner_async,
//     None,
// );