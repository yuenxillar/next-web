use std::fmt;

/// Interface for objects that are suitable for message resolution in a
/// [`MessageSource`].
///
/// Spring's own validation error classes implement this interface.
///
/// [`MessageSource`]: trait.MessageSource.html
pub trait MessageSourceResolvable {
    /// Return the codes to be used to resolve this message, in the order that
    /// they should get tried. The last code will therefore be the default one.
    ///
    /// # Returns
    /// A slice of codes which are associated with this message, or `None` if no
    /// codes are available.
    fn codes(&self) -> Option<&[String]>;

    /// Return the array of arguments to be used to resolve this message.
    ///
    /// The default implementation simply returns `None`.
    ///
    /// # Returns
    /// A slice of objects to be used as parameters to replace placeholders
    /// within the message text.
    fn arguments(&self) -> Option<&[Box<dyn fmt::Display>]> {
        None
    }

    /// Return the default message to be used to resolve this message.
    ///
    /// The default implementation simply returns `None`.
    /// Note that the default message may be identical to the primary
    /// message code ([`get_codes`]), which effectively enforces
    /// `AbstractMessageSource#setUseCodeAsDefaultMessage` for this
    /// particular message.
    ///
    /// # Returns
    /// The default message, or `None` if no default.
    ///
    /// [`get_codes`]: #method.get_codes
    fn default_message(&self) -> Option<&str> {
        None
    }
}
