//! Utilities for working with strings that have placeholder values in them.
//!
//! The [`PlaceholderParser`] resolves `${name}` style placeholders (with configurable
//! prefix, suffix, separator and escape characters) using a [`PlaceholderResolver`].
//! [`PropertyPlaceholderHelper`] is a convenience wrapper around the parser.

mod error;
mod placeholder_parser;
mod property_placeholder_helper;

pub use error::{BoxError, PlaceholderResolutionError};
pub use placeholder_parser::PlaceholderParser;
pub use property_placeholder_helper::{
    MapPlaceholderResolver, PlaceholderResolver, PropertyPlaceholderHelper,
    SharedPropertyPlaceholderHelper,
};
