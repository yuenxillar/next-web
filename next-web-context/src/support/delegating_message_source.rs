//! Port of `org.springframework.context.support.DelegatingMessageSource`.
//!
//! An empty [`MessageSource`] that delegates every call to its parent. Without a
//! parent it resolves no code at all: the only message it can produce is the
//! default message a caller provides, which is rendered with the argument syntax
//! of [`MessageFormat`](crate::util::MessageFormat).
//!
//! The type is used as a placeholder by the application context when the context
//! does not define a message source of its own, so that the message resolution
//! of a child context reaches the parent context. It is not meant to be used
//! directly by an application.
//!
//! # Examples
//!
//! ```ignore
//! let mut message_source = DelegatingMessageSource::default();
//! assert_eq!(message_source.message_or_default("greeting", None, Some("Hello"), None),
//!            Some("Hello".to_owned()));
//!
//! message_source.set_parent_message_source(parent);
//! assert_eq!(message_source.message("greeting", None, None), parent_message);
//! ```

use std::fmt;
use std::sync::Arc;

use crate::util::MessageFormat;
use crate::{Locale, MessageSource, MessageSourceResolvable, NoSuchMessageError};

/// A [`MessageSource`] that delegates all calls to its parent message source.
///
/// The type is the Rust counterpart of
/// `org.springframework.context.support.DelegatingMessageSource`, together with
/// the message rendering of its superclass `MessageSourceSupport`.
///
/// Its behavior depends on whether a parent is installed:
///
/// - with a parent, every call is delegated to it as it is, including the
///   default message a caller provided;
/// - without a parent, a code cannot be resolved: a provided default message is
///   rendered, and a lookup without a default message fails with a
///   [`NoSuchMessageError`].
#[derive(Clone, Default)]
pub struct DelegatingMessageSource {
    parent_message_source: Option<Arc<dyn MessageSource>>,
    always_use_message_format: bool,
}

impl DelegatingMessageSource {
    /// Creates a message source that delegates to the given parent.
    ///
    /// # Arguments
    ///
    /// * `parent_message_source` - The message source every call is delegated
    ///   to, or `None` for a message source that resolves no code.
    pub fn new(parent_message_source: Option<Arc<dyn MessageSource>>) -> Self {
        Self {
            parent_message_source,
            always_use_message_format: false,
        }
    }

    /// Sets the message source every call is delegated to.
    ///
    /// Equivalent to `setParentMessageSource(MessageSource)`.
    ///
    /// # Arguments
    ///
    /// * `parent_message_source` - The parent message source.
    pub fn set_parent_message_source(&mut self, parent_message_source: Arc<dyn MessageSource>) {
        self.parent_message_source = Some(parent_message_source);
    }

    /// Clears the message source every call is delegated to.
    ///
    /// Equivalent to `setParentMessageSource(null)`: the message source then
    /// resolves no code.
    pub fn clear_parent_message_source(&mut self) {
        self.parent_message_source = None;
    }

    /// Returns the message source every call is delegated to.
    ///
    /// Equivalent to `getParentMessageSource()`.
    pub fn parent_message_source(&self) -> Option<&Arc<dyn MessageSource>> {
        self.parent_message_source.as_ref()
    }

    /// Returns whether a default message is always rendered as a
    /// [`MessageFormat`], and not only when it declares an argument.
    ///
    /// Equivalent to `isAlwaysUseMessageFormat()`.
    pub fn is_always_use_message_format(&self) -> bool {
        self.always_use_message_format
    }

    /// Sets whether a default message is always rendered as a
    /// [`MessageFormat`].
    ///
    /// Equivalent to `setAlwaysUseMessageFormat(boolean)`. The default is
    /// `false`, which means that the pattern of a default message is only
    /// applied when the call provides arguments.
    ///
    /// # Arguments
    ///
    /// * `always_use_message_format` - Whether the pattern of a default message
    ///   is always applied.
    pub fn set_always_use_message_format(&mut self, always_use_message_format: bool) {
        self.always_use_message_format = always_use_message_format;
    }

    /// Renders a default message with the given arguments.
    ///
    /// Equivalent to `MessageSourceSupport.renderDefaultMessage(String, Object[], Locale)`:
    /// the pattern of the default message is only applied when it has to be.
    ///
    /// # Arguments
    ///
    /// * `default_message` - The default message to render.
    /// * `args` - The arguments the default message is rendered with.
    fn render_default_message(&self, default_message: &str, args: &[&dyn fmt::Display]) -> String {
        if !self.always_use_message_format && args.is_empty() {
            return default_message.to_owned();
        }

        MessageFormat::new(default_message).format(args)
    }
}

impl MessageSource for DelegatingMessageSource {
    fn message_or_default(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        if let Some(parent_message_source) = self.parent_message_source.as_deref() {
            return parent_message_source.message_or_default(code, args, default_message, locale);
        }

        default_message.map(|default_message| self.render_default_message(default_message, args))
    }

    fn message(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        if let Some(parent_message_source) = self.parent_message_source.as_deref() {
            return parent_message_source.message(code, args, locale);
        }

        // Without a parent the message source is empty: only the caller knows
        // how the code could be resolved.
        Err(NoSuchMessageError::new(code, locale))
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        if let Some(parent_message_source) = self.parent_message_source.as_deref() {
            return parent_message_source.message_from_resolvable(resolvable, locale);
        }

        let arguments = resolvable
            .arguments()
            .map(|arguments| arguments.iter().map(AsRef::as_ref).collect::<Vec<_>>());

        if let Some(default_message) = resolvable.default_message() {
            return Ok(self.render_default_message(
                default_message,
                arguments.as_deref().unwrap_or_default(),
            ));
        }

        // The original reports the first code of the resolvable here, where the
        // abstract message source reports the last one.
        let code = resolvable
            .codes()
            .and_then(|codes| codes.first())
            .map(String::as_str)
            .unwrap_or_default();

        Err(NoSuchMessageError::new(code, locale))
    }
}

impl fmt::Debug for DelegatingMessageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DelegatingMessageSource")
            .field(
                "parent_message_source",
                &self.parent_message_source.is_some(),
            )
            .field("always_use_message_format", &self.always_use_message_format)
            .finish()
    }
}

impl fmt::Display for DelegatingMessageSource {
    /// Writes the parent of this message source, the way `toString()` does.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.parent_message_source.as_deref() {
            Some(parent_message_source) => write!(f, "{parent_message_source:?}"),
            None => f.write_str("Empty MessageSource"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A message source that resolves every code to a fixed message.
    #[derive(Debug)]
    struct TestMessageSource {
        message: String,
    }

    impl TestMessageSource {
        fn new(message: &str) -> Arc<Self> {
            Arc::new(Self {
                message: message.to_owned(),
            })
        }
    }

    impl MessageSource for TestMessageSource {
        fn message_or_default(
            &self,
            _code: &str,
            _args: &[&dyn fmt::Display],
            _default_message: Option<&str>,
            _locale: Option<&Locale>,
        ) -> Option<String> {
            Some(self.message.clone())
        }

        fn message(
            &self,
            _code: &str,
            _args: &[&dyn fmt::Display],
            _locale: Option<&Locale>,
        ) -> Result<String, NoSuchMessageError> {
            Ok(self.message.clone())
        }

        fn message_from_resolvable(
            &self,
            _resolvable: &dyn MessageSourceResolvable,
            _locale: Option<&Locale>,
        ) -> Result<String, NoSuchMessageError> {
            Ok(self.message.clone())
        }
    }

    /// A resolvable with the given codes, default message and arguments.
    struct TestResolvable {
        codes: Vec<String>,
        default_message: Option<String>,
        arguments: Vec<Box<dyn fmt::Display>>,
    }

    impl TestResolvable {
        fn new(codes: &[&str], default_message: Option<&str>, arguments: &[&str]) -> Self {
            Self {
                codes: codes.iter().map(|code| (*code).to_owned()).collect(),
                default_message: default_message.map(str::to_owned),
                arguments: arguments
                    .iter()
                    .map(|argument| Box::new((*argument).to_owned()) as Box<dyn fmt::Display>)
                    .collect(),
            }
        }
    }

    impl MessageSourceResolvable for TestResolvable {
        fn codes(&self) -> Option<&[String]> {
            Some(&self.codes)
        }

        fn arguments(&self) -> Option<&[Box<dyn fmt::Display>]> {
            (!self.arguments.is_empty()).then_some(&self.arguments)
        }

        fn default_message(&self) -> Option<&str> {
            self.default_message.as_deref()
        }
    }

    #[test]
    fn without_a_parent_only_a_default_message_is_resolved() {
        let message_source = DelegatingMessageSource::default();

        assert_eq!(
            message_source.message_or_default("greeting", &[], None, None),
            None
        );
        assert_eq!(
            message_source.message_or_default("greeting", &[], Some("Hello"), None),
            Some("Hello".to_owned())
        );
        assert_eq!(
            message_source.message_or_default("greeting", &[&"Rust"], Some("Hello {0}"), None),
            Some("Hello Rust".to_owned())
        );

        let error = message_source
            .message("greeting", &[], Some(&Locale::new("zh", "CN")))
            .unwrap_err();
        assert_eq!(error.code, "greeting");
        assert_eq!(error.locale, Some(Locale::new("zh", "CN")));
    }

    #[test]
    fn without_a_parent_a_resolvable_resolves_its_default_message() {
        let message_source = DelegatingMessageSource::default();

        let resolvable = TestResolvable::new(&["greeting", "hello"], None, &[]);
        let error = message_source
            .message_from_resolvable(&resolvable, None)
            .unwrap_err();
        // The original reports the first code of the resolvable.
        assert_eq!(error.code, "greeting");

        let resolvable = TestResolvable::new(&["greeting"], Some("Hello {0}"), &["Rust"]);
        assert_eq!(
            message_source
                .message_from_resolvable(&resolvable, None)
                .unwrap(),
            "Hello Rust"
        );

        let resolvable = TestResolvable::new(&[], None, &[]);
        assert_eq!(
            message_source
                .message_from_resolvable(&resolvable, None)
                .unwrap_err()
                .code,
            ""
        );
    }

    #[test]
    fn delegates_every_call_to_the_parent() {
        let mut message_source = DelegatingMessageSource::default();
        message_source.set_parent_message_source(TestMessageSource::new("from the parent"));

        assert!(message_source.parent_message_source().is_some());
        assert_eq!(
            message_source.message_or_default("greeting", &[], Some("Hello"), None),
            Some("from the parent".to_owned())
        );
        assert_eq!(
            message_source.message("greeting", &[], None).unwrap(),
            "from the parent"
        );
        assert_eq!(
            message_source
                .message_from_resolvable(&TestResolvable::new(&["greeting"], None, &[]), None)
                .unwrap(),
            "from the parent"
        );
        assert_ne!(message_source.to_string(), "Empty MessageSource");

        message_source.clear_parent_message_source();
        assert_eq!(message_source.to_string(), "Empty MessageSource");
    }

    #[test]
    fn applies_the_pattern_of_a_default_message_when_asked_to() {
        let mut message_source = DelegatingMessageSource::default();

        // Without the flag the pattern of the default message is only applied
        // when the call provides arguments.
        assert_eq!(
            message_source.message_or_default("greeting", &[], Some("''{0}''"), None),
            Some("''{0}''".to_owned())
        );

        message_source.set_always_use_message_format(true);
        assert!(message_source.is_always_use_message_format());
        assert_eq!(
            message_source.message_or_default("greeting", &[], Some("''{0}''"), None),
            Some("'{0}'".to_owned())
        );
    }
}
