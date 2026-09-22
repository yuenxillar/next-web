#![allow(missing_docs)]
pub mod event;
pub mod autoregister;
pub mod provider;
pub mod support;

mod application_context;
mod application_context_ext;
mod application_event;
mod application_event_publisher;
mod application_listener;
mod message_source;
mod message_source_resolvable;
mod no_such_message_error;

pub use application_context::{
    ApplicationContext, InstanceClone, default_singleton_name,
};
pub use application_context_ext::ApplicationContextExt;
pub use autoregister::{
    AutoRegisterModule, ProviderRegister, auto_registered_providers, submit,
};
pub use provider::{
    BoxValue, Color, Constructor, DefaultProvider, Definition, DynProvider, EagerCreateFunction,
    FutureExt, Module, Provider, ResolveModule, Scope, SingleOwnerAsyncProvider,
    SingleOwnerProvider, SingletonAsyncProvider, SingletonProvider, TransientAsyncProvider,
    TransientProvider, single_owner, single_owner_async, singleton, singleton_async, transient,
    transient_async,
};
pub use application_event::{ApplicationEvent, EventAttributes};
pub use application_event_publisher::ApplicationEventPublisher;
pub use application_listener::ApplicationListener;
pub use message_source::MessageSource;
pub use message_source_resolvable::MessageSourceResolvable;
pub use no_such_message_error::NoSuchMessageError;

#[derive(Debug, Clone)]
pub struct Locale;

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "en")
    }
}

pub(crate) type BoxFuture<'a, T> =
    core::pin::Pin<std::boxed::Box<dyn Future<Output = T> + Send + 'a>>;
