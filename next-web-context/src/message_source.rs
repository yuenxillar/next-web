use std::fmt;

use crate::{Locale, MessageSourceResolvable, NoSuchMessageError};

/// Strategy trait for resolving messages, with support for the parameterization
/// and internationalization of such messages.
///
/// Implementations can be backed by various sources:
/// * `ResourceBundleMessageSource` - built on top of standard resource bundles
/// * `ReloadableResourceBundleMessageSource` - highly configurable with reloading support
pub trait MessageSource {
    /// Try to resolve the message. Return default message if no message was found.
    ///
    /// # Arguments
    /// * `code` - The message code to look up, e.g. 'calculator.noRateSet'.
    ///   Users are encouraged to base message names on qualified class
    ///   or package names, avoiding potential conflicts and ensuring maximum clarity.
    /// * `args` - An array of arguments that will be filled in for params within
    ///   the message (params look like "{0}", "{1,date}", "{2,time}" within a message),
    ///   or `None` if none.
    /// * `default_message` - A default message to return if the lookup fails.
    /// * `locale` - The locale in which to do the lookup.
    ///
    /// # Returns
    /// The resolved message if the lookup was successful,
    /// otherwise the default message passed as a parameter (which may be `None`).
    fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String>;

    /// Try to resolve the message. Treat as an error if the message can't be found.
    ///
    /// # Arguments
    /// * `code` - The message code to look up, e.g. 'calculator.noRateSet'.
    /// * `args` - An array of arguments that will be filled in for params within
    ///   the message, or `None` if none.
    /// * `locale` - The locale in which to do the lookup.
    ///
    /// # Returns
    /// The resolved message (never `None`).
    ///
    /// # Errors
    /// Returns `NoSuchMessageError` if no corresponding message was found.
    fn message(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError>;

    /// Try to resolve the message using all the attributes contained within the
    /// `MessageSourceResolvable` argument that was passed in.
    ///
    /// NOTE: We must return a `Result` on this method since at the time of
    /// calling this method we aren't able to determine if the `default_message`
    /// property of the resolvable is `None` or not.
    ///
    /// # Arguments
    /// * `resolvable` - The value object storing attributes required to resolve a message
    ///   (may include a default message).
    /// * `locale` - The locale in which to do the lookup.
    ///
    /// # Returns
    /// The resolved message (never `None` since even a `MessageSourceResolvable`-provided
    /// default message needs to be non-null).
    ///
    /// # Errors
    /// Returns `NoSuchMessageError` if no corresponding message was found
    /// (and no default message was provided by the `MessageSourceResolvable`).
    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError>;
}
