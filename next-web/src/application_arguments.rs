//! Provides access to the arguments that were used to run a [`NextWebApplication`].
//!
//! This trait mirrors the `ApplicationArguments` interface, offering a typed and
//! idiomatic way to inspect both option and non-option arguments passed to an
//! application at startup.

use std::collections::HashSet;

/// Provides access to the arguments that were used to run an application.
///
/// Implementors of this trait expose the raw arguments as well as helpers for
/// inspecting option arguments (e.g. `--foo=bar`) and non-option arguments.
pub trait ApplicationArguments {
    /// Returns the raw unprocessed arguments that were passed to the application.
    ///
    /// # Returns
    ///
    /// A slice containing the original arguments.
    fn source_args(&self) -> &[String];

    /// Returns the names of all option arguments.
    ///
    /// For example, if the arguments were `--foo=bar --debug`, this returns the
    /// values `["foo", "debug"]`.
    ///
    /// # Returns
    ///
    /// A set of option names, or an empty set if there are none.
    fn option_names(&self) -> HashSet<&str>;

    /// Returns whether the set of option arguments parsed from the arguments
    /// contains an option with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to check.
    ///
    /// # Returns
    ///
    /// `true` if the arguments contain an option with the given name.
    fn contains_option(&self, name: &str) -> bool;

    /// Returns the collection of values associated with the option argument having
    /// the given name.
    ///
    /// * If the option is present and has no argument (e.g. `--foo`), an empty
    ///   collection is returned.
    /// * If the option is present and has a single value (e.g. `--foo=bar`), a
    ///   collection having one element is returned.
    /// * If the option is present and has multiple values (e.g.
    ///   `--foo=bar --foo=baz`), a collection having an element for each value is
    ///   returned.
    /// * If the option is not present, `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the option.
    ///
    /// # Returns
    ///
    /// An optional list of option values for the given name.
    fn option_values(&self, name: &str) -> Option<&[String]>;

    /// Returns the collection of non-option arguments parsed.
    ///
    /// # Returns
    ///
    /// A slice of non-option arguments, or an empty slice if there are none.
    fn non_option_args(&self) -> &[String];
}
