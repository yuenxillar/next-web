pub mod any;
pub mod config;
pub mod constants;
pub mod context;
pub mod env;
pub mod error;
pub mod http;
pub mod io;
pub mod messaging;
pub mod metrics;
pub mod mime_type;
pub mod security;
pub mod server;
pub mod service;
pub mod util;
pub mod web;

// The dependency injection service provider interface lives in its own, light
// crate, so that a library that only contributes singletons does not have to
// depend on the runtime. It is re-exported here for convenience.
pub use next_web_context::{
    ApplicationContext, AutoRegisterModule, BoxValue, Color, Constructor, DefaultProvider,
    Definition, DynProvider, EagerCreateFunction, FutureExt, Module, Provider, ProviderRegister,
    ResolveModule, Scope, auto_registered_providers, register_provider, submit,
};

pub use arc_swap::*;
pub use async_trait::async_trait;
pub use dyn_clone::{DynClone, clone_box, clone_trait_object};
pub use regex::{Regex, RegexBuilder};
// The template engine the framework renders the views of an application with.
// It is re-exported so a handler can name `next_web_core::tera::Context`
// without depending on the version of the crate the framework uses.
pub use tera;
pub use uuid::Uuid;

/// A thread-safe, shared, type-erased value.
///
/// Equivalent to `Arc<dyn Any + Send + Sync>`. Useful for storing values
/// of arbitrary types in shared containers (e.g., dependency injection
/// registries, plugin systems, or event buses) that need to be accessed
/// from multiple threads.
pub type AnyObject = std::sync::Arc<dyn std::any::Any + Send + Sync>;

/// A thread-safe, uniquely-owned, type-erased value.
///
/// Equivalent to `Box<dyn Any + Send + Sync>`. Useful when a value of an
/// arbitrary type needs to be boxed and transferred by ownership, such as
/// storing it in a collection and later downcasting it back to its
/// concrete type.
pub type BoxAny = std::boxed::Box<dyn std::any::Any + Send + Sync>;

/// A boxed, pinned, thread-safe future with lifetime `'a`.
///
/// Equivalent to `Pin<Box<dyn Future<Output = T> + Send + 'a>>`. Commonly
/// used as the return type of trait methods that need to be `async`,
/// since `async fn` in traits is not directly supported in all cases.
///
/// The `Send` bound allows the future to be moved across threads. Remove
/// it if the future does not need to be `Send`.
pub type BoxFuture<'a, T> =
    core::pin::Pin<std::boxed::Box<dyn core::future::Future<Output = T> + Send + 'a>>;

mod apply_router;
mod id;
mod named;
mod next_version;
mod ordered;
mod required;

pub use apply_router::ApplyRouter;
pub use id::Id;
pub use named::Named;
pub use next_version::NextVersion;
pub use ordered::Ordered;
pub use required::Required;
