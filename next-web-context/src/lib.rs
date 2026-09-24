#![allow(missing_docs)]
pub mod autoregister;
pub mod event;
pub mod i18n;
pub mod provider;
pub mod support;
pub mod util;

mod application_context;
mod application_context_ext;
mod application_event;
mod application_event_publisher;
mod application_listener;
mod condition;
mod message_source;
mod message_source_resolvable;
mod no_such_message_error;

pub use application_context::{
    APPLICATION_ENVIRONMENT_SINGLETON_NAME, APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
    ApplicationContext, InstanceClone, MESSAGE_SOURCE_SINGLETON_NAME,
    RESOURCE_LOADER_SINGLETON_NAME, default_singleton_name,
};
pub use application_context_ext::ApplicationContextExt;
pub use application_event::{ApplicationEvent, EventAttributes};
pub use application_event_publisher::ApplicationEventPublisher;
pub use application_listener::ApplicationListener;
pub use autoregister::{AutoRegisterModule, ProviderRegister, auto_registered_providers, submit};
pub use condition::Condition;
pub use message_source::MessageSource;
pub use message_source_resolvable::MessageSourceResolvable;
pub use no_such_message_error::NoSuchMessageError;
pub use provider::{
    BoxValue, Color, Constructor, DefaultProvider, Definition, DynProvider, EagerCreateFunction,
    FutureExt, Module, Provider, ResolveModule, Scope, SingleOwnerAsyncProvider,
    SingleOwnerProvider, SingletonAsyncProvider, SingletonProvider, TransientAsyncProvider,
    TransientProvider, single_owner, single_owner_async, singleton, singleton_async, transient,
    transient_async,
};
pub use util::Locale;

pub(crate) type BoxFuture<'a, T> =
    core::pin::Pin<std::boxed::Box<dyn Future<Output = T> + Send + 'a>>;
