//! Additional information about a [`PropertySource`].
//!
//! [`PropertySource`]: next_web_core::env::PropertySource

/// Describes a property source that carries information about itself, beyond
/// the properties it exposes.
///
/// A source is *immutable* when none of its properties can change after the
/// source has been created. Consumers can use that information, for example, to
/// decide whether a source has to be re-read when the environment changes.
pub trait PropertySourceInfo {
    /// Returns whether the property source is immutable.
    fn is_immutable(&self) -> bool;
}
