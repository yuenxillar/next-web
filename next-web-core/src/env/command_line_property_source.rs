//! Abstract base trait for property sources backed by command line arguments.
//!
//! The parameterized type `T` represents the underlying source of command line
//! options.

use std::ops::{Deref, DerefMut};

use crate::env::BasePropertySource;

/// The default name given to [`CommandLinePropertySource`] instances.
pub const COMMAND_LINE_PROPERTY_SOURCE_NAME: &str = "commandLineArgs";

/// The default name of the property representing non-option arguments.
pub const DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME: &str = "nonOptionArgs";

/// Abstract base trait for [`PropertySource`] implementations backed by command
/// line arguments.
///
/// # Purpose and General Usage
///
/// For use in standalone applications that are bootstrapped via a traditional
/// `main` function accepting the command line arguments. In many cases,
/// processing command-line arguments directly within `main` may be sufficient,
/// but in other cases it may be desirable to inject arguments as values into
/// application components.
///
/// A `CommandLinePropertySource` will typically be added to the
/// [`Environment`](crate::environment::Environment) of the application context,
/// at which point all command line arguments become available through the
/// [`Environment::get_property`](crate::environment::Environment::get_property)
/// family of methods.
///
/// Because the `CommandLinePropertySource` is usually added with the highest
/// precedence, arguments specified on the command line are naturally more
/// specific than those specified as environment variables.
///
/// # Working with option arguments
///
/// Individual command line arguments are represented as properties. For
/// example, given the following command line:
///
/// ```text
/// --o1=v1 --o2
/// ```
///
/// `o1` and `o2` are treated as "option arguments", and the following
/// assertions would evaluate to `true`:
///
/// ```ignore
/// assert!(ps.contains_property("o1"));
/// assert!(ps.contains_property("o2"));
/// assert!(!ps.contains_property("o3"));
/// assert_eq!(ps.get_property("o1"), Some("v1".to_string()));
/// assert_eq!(ps.get_property("o2"), Some(String::new()));
/// assert_eq!(ps.get_property("o3"), None);
/// ```
///
/// Note that the `o2` option has no argument, but `get_property("o2")` resolves
/// to an empty string as opposed to `None`, while `get_property("o3")` resolves
/// to `None` because it was not specified. This behavior is consistent with the
/// general contract followed by all [`PropertySource`] implementations.
///
/// Note also that while `--` was used in the examples above to denote an option
/// argument, this syntax may vary across individual command line argument
/// libraries.
///
/// # Working with non-option arguments
///
/// Non-option arguments are also supported through this abstraction. Any
/// arguments supplied without an option-style prefix such as `-` or `--` are
/// considered "non-option arguments" and available through the special
/// [`DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME`] property. If multiple non-option
/// arguments are specified, the value of this property will be a
/// comma-delimited string containing all the arguments. This approach ensures a
/// simple and consistent return type (`String`) for all properties from a
/// `CommandLinePropertySource`.
///
/// Consider the following example:
///
/// ```text
/// --o1=v1 --o2=v2 /path/to/file1 /path/to/file2
/// ```
///
/// In this example, `o1` and `o2` would be considered "option arguments",
/// while the two filesystem paths qualify as "non-option arguments". As such,
/// the following assertions will evaluate to `true`:
///
/// ```ignore
/// assert!(ps.contains_property("o1"));
/// assert!(ps.contains_property("o2"));
/// assert!(ps.contains_property("nonOptionArgs"));
/// assert_eq!(ps.get_property("o1"), Some("v1".to_string()));
/// assert_eq!(ps.get_property("o2"), Some("v2".to_string()));
/// assert_eq!(
///     ps.get_property("nonOptionArgs"),
///     Some("/path/to/file1,/path/to/file2".to_string())
/// );
/// ```
///
/// The name of the special "non-option arguments" property may be customized
/// through [`set_non_option_args_property_name`](Self::set_non_option_args_property_name).
/// Doing so is recommended as it gives proper semantic value to non-option
/// arguments.
///
/// # Limitations
///
/// This abstraction is not intended to expose the full power of underlying
/// command line parsing APIs. Its intent is rather just the opposite: to
/// provide the simplest possible abstraction for accessing command line
/// arguments *after* they have been parsed.
///
/// # Type Parameters
///
/// * `T` - the source type, exposed through the underlying
///   [`EnumerablePropertySource`].
#[derive(Clone)]
pub struct CommandLinePropertySource<T> {
    non_option_args_property_name: String,
    base: BasePropertySource<T>,
}

impl<T> CommandLinePropertySource<T> {
    /// Creates a new [`CommandLinePropertySource`] with the given name and source.
    ///
    /// # Arguments
    ///
    /// * `name` - the name of the property source.
    /// * `source` - the source of the property values.
    pub fn new(name: impl Into<String>, source: T) -> Self {
        Self {
            non_option_args_property_name: DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME.to_owned(),
            base: BasePropertySource::new(name.into(), source),
        }
    }

    /// Creates a new [`CommandLinePropertySource`] with the given source.
    ///
    /// # Arguments
    ///
    /// * `source` - the source of the property values.
    pub fn with_source(source: T) -> Self {
        Self {
            non_option_args_property_name: DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME.to_owned(),
            base: BasePropertySource::new(COMMAND_LINE_PROPERTY_SOURCE_NAME, source),
        }
    }

    /// Sets the name of the special "non-option arguments" property.
    ///
    /// The default is [`DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME`].
    ///
    /// # Arguments
    ///
    /// * `non_option_args_property_name` - The property name to use for
    ///   non-option arguments.
    pub fn set_non_option_args_property_name(
        &mut self,
        non_option_args_property_name: impl Into<String>,
    ) {
        self.non_option_args_property_name = non_option_args_property_name.into();
    }

    /// Returns whether the property source contains a property with the given
    /// name.
    ///
    /// This implementation first checks whether the name specified is the
    /// special "non-option arguments" property, and if so delegates to
    /// [`Self::non_option_args`], checking whether it returns an empty
    /// collection. Otherwise, delegates to and returns the value of
    /// [`Self::contains_option`].
    ///
    /// # Arguments
    ///
    /// * `name` - The property name to check.
    ///
    /// # Returns
    ///
    /// `true` if the property is present.
    pub fn contains_property(
        &self,
        name: &str,
        support: &dyn CommandLinePropertySourceExt,
    ) -> bool {
        if self.non_option_args_property_name == name {
            return !support.non_option_args().is_empty();
        }
        support.contains_option(name)
    }

    /// Returns the property value for the given name.
    ///
    /// This implementation first checks whether the name specified is the
    /// special "non-option arguments" property, and if so delegates to
    /// [`Self::non_option_args`]. If the collection of non-option arguments is
    /// empty, this method returns `None`. If not empty, it returns a
    /// comma-separated string of all non-option arguments. Otherwise, this
    /// method delegates to and returns a comma-separated string of the results
    /// of [`Self::option_values`], or `None` if there are no such option
    /// values.
    ///
    /// # Arguments
    ///
    /// * `name` - The property name.
    ///
    /// # Returns
    ///
    /// The property value, or `None` if not present.
    pub fn property(
        &self,
        name: &str,
        support: &dyn CommandLinePropertySourceExt,
    ) -> Option<String> {
        if self.non_option_args_property_name == name {
            let non_option_args = support.non_option_args();
            if non_option_args.is_empty() {
                return None;
            }
            return Some(collection_to_comma_delimited_string(&non_option_args));
        }
        let option_values = support.option_values(name)?;
        Some(collection_to_comma_delimited_string(&option_values))
    }
}

impl<T> Deref for CommandLinePropertySource<T> {
    type Target = BasePropertySource<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T> DerefMut for CommandLinePropertySource<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Extends [`CommandLinePropertySource`] with convenience methods for accessing
/// option values and non-option arguments.
pub trait CommandLinePropertySourceExt {
    /// Returns whether the underlying option arguments contain an option with
    /// the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The option name to check.
    ///
    /// # Returns
    ///
    /// `true` if an option with the given name is present.
    fn contains_option(&self, name: &str) -> bool;

    /// Returns the collection of values associated with the command line option
    /// having the given name.
    ///
    /// * If the option is present and has no argument (for example, `--foo`),
    ///   returns an empty collection.
    /// * If the option is present and has a single value (for example,
    ///   `--foo=bar`), returns a collection having one element.
    /// * If the option is present and the underlying command line parsing
    ///   library supports multiple arguments (for example,
    ///   `--foo=bar --foo=baz`), returns a collection having an element for
    ///   each value.
    /// * If the option is not present, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `name` - The option name.
    ///
    /// # Returns
    ///
    /// An optional list of option values.
    fn option_values(&self, name: &str) -> Option<&[String]>;

    /// Returns the collection of non-option arguments parsed from the command
    /// line.
    ///
    /// # Returns
    ///
    /// A list of non-option arguments. Never `None`; may be empty.
    fn non_option_args(&self) -> &[String];
}

/// Joins the given collection into a comma-delimited string.
///
/// This is a small utility mirroring the original
/// `StringUtils.collectionToCommaDelimitedString` behavior used by
/// [`CommandLinePropertySource`].
///
/// # Arguments
///
/// * `values` - The values to join.
///
/// # Returns
///
/// A comma-delimited string representation of the values.
fn collection_to_comma_delimited_string(values: &[String]) -> String {
    values.join(",")
}
