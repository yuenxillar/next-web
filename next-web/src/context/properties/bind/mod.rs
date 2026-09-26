//! Binding the properties of an environment to a type of the application.
//!
//! The binder itself lives in [`next_web_core::env::bind`], so that a library
//! that only depends on the core crate can bind the properties it declares. It
//! is re-exported here, where the configuration properties of the application
//! are looked for.

mod placeholders_resolver;

pub use placeholders_resolver::{NoOpPlaceholdersResolver, PlaceholdersResolver};

pub use next_web_core::env::bind::{
    BindError, Binder, ConfigurableEnvironmentBindExt,
};
