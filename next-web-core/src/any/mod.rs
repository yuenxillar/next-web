//! Type-erased storage primitives.
//!
//! This module groups the small, self-contained building blocks used to store
//! heterogeneous data: cloneable values ([`AnyClone`]), hashable errors
//! ([`AnyError`]), and two map flavors ([`AnyMap`] for async access and
//! [`AnyDashMap`] for synchronous, sharded access).

pub mod any_clone;
pub mod any_error;
pub mod any_map;
pub mod any_value;

pub use any_clone::AnyClone;
pub use any_error::AnyError;
pub use any_map::{AnyDashMap, AnyMap};
pub use any_value::{AnyValue, Number};
