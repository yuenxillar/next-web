//! Binding the properties of an environment to a type.
//!
//! A [`Binder`] reads the properties below a prefix from a
//! [`ConfigurableEnvironment`] and deserializes them into the type that is
//! asked for, so that the value of the property `next.messages.base_name`
//! becomes the value of the field `base_name` of the `next.messages` prefix.
//!
//! The deserializer of a binding resolves the properties through the
//! environment, which searches its property sources in order and replaces the
//! placeholders of the values it resolves. The environment variables of the
//! process and the arguments of the command line therefore take part in a
//! binding like the properties of a property file do. A field is bound from the
//! first of its names the environment resolves: the name of the field, its
//! dashed form, and the name of an environment variable.
//!
//! [`ConfigurableEnvironment`]: crate::env::ConfigurableEnvironment

mod bind_error;
mod binder;
mod property_deserializer;
mod property_index;
mod property_tree;
mod value_deserializer;

pub use bind_error::BindError;
pub use binder::{Binder, ConfigurableEnvironmentBindExt};

pub(crate) use value_deserializer::PropertyValueDeserializer;
