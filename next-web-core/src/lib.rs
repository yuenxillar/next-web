pub mod anys;
pub mod autoconfigure;
pub mod autoregister;
pub mod client;
pub mod common;
pub mod constants;
pub mod context;
pub mod convert;
pub mod cors;
pub mod env;
pub mod error;
pub mod filter;
pub mod http;
pub mod io;
pub mod macros;
pub mod messaging;
pub mod metrics;
pub mod mime_type;
pub mod server;
pub mod store;
pub mod traits;
pub mod util;
pub mod web;

pub use self::autoregister::auto_register::*;

// The dependency injection service provider interface lives in its own, light
// crate, so that a library that only contributes singletons does not have to
// depend on the runtime. It is re-exported here for convenience.
pub use next_web_context::{
    ApplicationContext, AutoRegisterModule, BoxValue, Color, Constructor, DefaultProvider,
    Definition, DynProvider, EagerCreateFunction, FutureExt, Module, Provider, ProviderRegister,
    ResolveModule, Scope, auto_registered_providers, register_provider,
};

pub use arc_swap::*;
pub use async_trait::async_trait;
pub use dyn_clone::{DynClone, clone_box, clone_trait_object};

#[cfg(feature = "http-request")]
pub extern crate headers;

pub use urlencoding;

pub type AnyObject = std::sync::Arc<dyn std::any::Any + Send + Sync>;
pub type BoxAny = std::boxed::Box<dyn std::any::Any + Send + Sync>;
pub type BoxFuture<'a, T> = core::pin::Pin<std::boxed::Box<dyn Future<Output = T> + Send + 'a>>;

mod next_version;
mod ordered;

pub use next_version::NextVersion;
pub use ordered::Ordered;
