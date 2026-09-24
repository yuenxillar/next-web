//! Port of `org.springframework.context.support.MessageSourceAccessor`.
//!
//! Helper class for easy access to messages from a [`MessageSource`], providing
//! the lookups of the original without its overloading: the lookup of a locale
//! that is not given answers with the [default locale](MessageSourceAccessor::default_locale)
//! of the accessor.
//!
//! The type is available from an application object, and it is also reusable as
//! a standalone helper an application object delegates to.

use std::fmt;
use std::sync::Arc;

use crate::i18n::LocaleContextHolder;
use crate::{Locale, MessageSource, MessageSourceResolvable, NoSuchMessageError};

/// Easy access to the messages of a [`MessageSource`].
///
/// The type is the counterpart of `MessageSourceAccessor`, and it wraps any
/// message source so that the callers of an application object do not have to
/// pass the same locale over and over.
#[derive(Clone, Debug)]
pub struct MessageSourceAccessor {
    message_source: Arc<dyn MessageSource>,
    default_locale: Option<Locale>,
}

impl MessageSourceAccessor {
    /// Creates an accessor using the locale of the [`LocaleContextHolder`] as
    /// its default locale.
    ///
    /// Equivalent to `MessageSourceAccessor(MessageSource)`.
    ///
    /// # Arguments
    ///
    /// * `message_source` - The message source the messages are read from.
    pub fn new(message_source: Arc<dyn MessageSource>) -> Self {
        Self {
            message_source,
            default_locale: None,
        }
    }

    /// Creates an accessor using the given default locale.
    ///
    /// Equivalent to `MessageSourceAccessor(MessageSource, Locale)`.
    ///
    /// # Arguments
    ///
    /// * `message_source` - The message source the messages are read from.
    /// * `default_locale` - The locale a lookup without a locale uses.
    pub fn with_default_locale(
        message_source: Arc<dyn MessageSource>,
        default_locale: Locale,
    ) -> Self {
        Self {
            message_source,
            default_locale: Some(default_locale),
        }
    }

    /// Returns the message source of this accessor.
    pub fn message_source(&self) -> &Arc<dyn MessageSource> {
        &self.message_source
    }

    /// Returns the locale a lookup without an explicit locale uses.
    ///
    /// Equivalent to `getDefaultLocale()`: the locale given to the constructor
    /// of the accessor wins over the locale of the [`LocaleContextHolder`].
    pub fn default_locale(&self) -> Locale {
        self.default_locale
            .clone()
            .unwrap_or_else(LocaleContextHolder::locale)
    }

    /// Resolves the message of the given code with the given arguments.
    ///
    /// Equivalent to `getMessage(String, Object[], String)`. A lookup that
    /// resolves no message answers with the default message; the accessor
    /// answers with an empty string when the message source resolves neither.
    ///
    /// # Arguments
    ///
    /// * `code` - The code of the message.
    /// * `args` - The arguments of the message, or `None` when it has none.
    /// * `default_message` - The message to answer with when the lookup fails.
    pub fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default_message: &str,
    ) -> String {
        self.message_or_default_with_locale(code, args, default_message, &self.default_locale())
    }

    /// Resolves the message of the given code with the given arguments and the
    /// given locale.
    ///
    /// Equivalent to `getMessage(String, Object[], String, Locale)`.
    ///
    /// # Arguments
    ///
    /// * `code` - The code of the message.
    /// * `args` - The arguments of the message, or `None` when it has none.
    /// * `default_message` - The message to answer with when the lookup fails.
    /// * `locale` - The locale the message is looked up in.
    pub fn message_or_default_with_locale(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default_message: &str,
        locale: &Locale,
    ) -> String {
        self.message_source
            .message_or_default(
                code,
                args.unwrap_or_default(),
                Some(default_message),
                Some(locale),
            )
            .unwrap_or_default()
    }

    /// Returns the message of the given code with the given arguments.
    ///
    /// Equivalent to `getMessage(String, Object[])`.
    ///
    /// # Arguments
    ///
    /// * `code` - The code of the message.
    /// * `args` - The arguments of the message, or `None` when it has none.
    ///
    /// # Errors
    ///
    /// Returns a [`NoSuchMessageError`] when the message source resolves no
    /// message for the code.
    pub fn message(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
    ) -> Result<String, NoSuchMessageError> {
        self.message_with_locale(code, args, &self.default_locale())
    }

    /// Returns the message of the given code with the given arguments and the
    /// given locale.
    ///
    /// Equivalent to `getMessage(String, Object[], Locale)`.
    ///
    /// # Arguments
    ///
    /// * `code` - The code of the message.
    /// * `args` - The arguments of the message, or `None` when it has none.
    /// * `locale` - The locale the message is looked up in.
    ///
    /// # Errors
    ///
    /// Returns a [`NoSuchMessageError`] when the message source resolves no
    /// message for the code.
    pub fn message_with_locale(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        locale: &Locale,
    ) -> Result<String, NoSuchMessageError> {
        self.message_source
            .message(code, args.unwrap_or_default(), Some(locale))
    }

    /// Returns the message of the given resolvable.
    ///
    /// Equivalent to `getMessage(MessageSourceResolvable)`.
    ///
    /// # Arguments
    ///
    /// * `resolvable` - The value object holding the codes, the arguments and
    ///   the default message of the lookup.
    ///
    /// # Errors
    ///
    /// Returns a [`NoSuchMessageError`] when the message source resolves no
    /// message for the codes of the resolvable and the resolvable declares no
    /// default message.
    pub fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
    ) -> Result<String, NoSuchMessageError> {
        self.message_from_resolvable_with_locale(resolvable, &self.default_locale())
    }

    /// Returns the message of the given resolvable in the given locale.
    ///
    /// Equivalent to `getMessage(MessageSourceResolvable, Locale)`.
    ///
    /// # Arguments
    ///
    /// * `resolvable` - The value object holding the codes, the arguments and
    ///   the default message of the lookup.
    /// * `locale` - The locale the message is looked up in.
    ///
    /// # Errors
    ///
    /// Returns a [`NoSuchMessageError`] when the message source resolves no
    /// message for the codes of the resolvable and the resolvable declares no
    /// default message.
    pub fn message_from_resolvable_with_locale(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: &Locale,
    ) -> Result<String, NoSuchMessageError> {
        self.message_source
            .message_from_resolvable(resolvable, Some(locale))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Returns the locale of the given language tag.
    fn locale(tag: &str) -> Locale {
        Locale::for_language_tag(tag).expect("the test locale is valid")
    }

    /// A message source that answers every code with the message it was given,
    /// and that remembers the locale and the arguments of the last lookup.
    #[derive(Debug, Default)]
    struct TestMessageSource {
        message: Option<String>,
        honours_default_message: bool,
        last_locale: Mutex<Option<Locale>>,
        last_arguments: Mutex<usize>,
    }

    impl TestMessageSource {
        /// Returns a source answering with the given message.
        fn with_message(message: Option<&str>) -> Arc<Self> {
            Arc::new(Self {
                message: message.map(str::to_owned),
                honours_default_message: true,
                ..Self::default()
            })
        }

        /// Returns a source that answers with nothing, even when it was given a
        /// default message.
        fn without_message() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn last_locale(&self) -> Option<Locale> {
            self.last_locale
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }

        fn last_arguments(&self) -> usize {
            *self
                .last_arguments
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }

        fn record(&self, locale: Option<&Locale>, arguments: usize) -> Option<String> {
            *self
                .last_locale
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = locale.cloned();
            *self
                .last_arguments
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = arguments;

            self.message.clone()
        }
    }

    impl MessageSource for TestMessageSource {
        fn message_or_default(
            &self,
            _code: &str,
            args: &[&dyn fmt::Display],
            default_message: Option<&str>,
            locale: Option<&Locale>,
        ) -> Option<String> {
            self.record(locale, args.len()).or_else(|| {
                self.honours_default_message
                    .then(|| default_message.map(str::to_owned))
                    .flatten()
            })
        }

        fn message(
            &self,
            code: &str,
            args: &[&dyn fmt::Display],
            locale: Option<&Locale>,
        ) -> Result<String, NoSuchMessageError> {
            self.record(locale, args.len())
                .ok_or_else(|| NoSuchMessageError::new(code, locale))
        }

        fn message_from_resolvable(
            &self,
            resolvable: &dyn MessageSourceResolvable,
            locale: Option<&Locale>,
        ) -> Result<String, NoSuchMessageError> {
            let arguments = resolvable
                .arguments()
                .map(|arguments| arguments.len())
                .unwrap_or_default();

            self.record(locale, arguments)
                .ok_or_else(|| NoSuchMessageError::new("resolvable", locale))
        }
    }

    /// A resolvable holding the given codes, arguments and default message.
    struct TestResolvable {
        codes: Vec<String>,
        arguments: Vec<Box<dyn fmt::Display>>,
        default_message: Option<String>,
    }

    impl TestResolvable {
        fn new(codes: &[&str], arguments: &[&str], default_message: Option<&str>) -> Self {
            Self {
                codes: codes.iter().map(|code| (*code).to_owned()).collect(),
                arguments: arguments
                    .iter()
                    .map(|argument| Box::new((*argument).to_owned()) as Box<dyn fmt::Display>)
                    .collect(),
                default_message: default_message.map(str::to_owned),
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
    fn the_locale_of_the_holder_is_the_default_locale() {
        LocaleContextHolder::set_locale(Some(locale("de-DE")));

        let message_source = TestMessageSource::with_message(Some("Guten Tag"));
        let accessor =
            MessageSourceAccessor::new(Arc::clone(&message_source) as Arc<dyn MessageSource>);

        assert_eq!(accessor.default_locale(), locale("de-DE"));
        assert_eq!(
            accessor.message_or_default("greeting", None, "fallback"),
            "Guten Tag"
        );
        assert_eq!(message_source.last_locale(), Some(locale("de-DE")));

        LocaleContextHolder::reset_locale_context();
    }

    #[test]
    fn the_configured_default_locale_wins_over_the_holder() {
        LocaleContextHolder::set_locale(Some(locale("de-DE")));

        let message_source = TestMessageSource::with_message(Some("こんにちは"));
        let accessor = MessageSourceAccessor::with_default_locale(
            Arc::clone(&message_source) as Arc<dyn MessageSource>,
            locale("ja-JP"),
        );

        assert_eq!(accessor.default_locale(), locale("ja-JP"));
        assert_eq!(
            accessor.message_or_default("greeting", None, "fallback"),
            "こんにちは"
        );
        assert_eq!(message_source.last_locale(), Some(locale("ja-JP")));

        LocaleContextHolder::reset_locale_context();
    }

    #[test]
    fn an_explicit_locale_wins_over_the_default_locale() {
        let message_source = TestMessageSource::with_message(Some("Hello"));
        let accessor = MessageSourceAccessor::with_default_locale(
            Arc::clone(&message_source) as Arc<dyn MessageSource>,
            locale("de-DE"),
        );

        assert_eq!(
            accessor.message_or_default_with_locale("greeting", None, "fallback", &locale("en-US")),
            "Hello"
        );
        assert_eq!(message_source.last_locale(), Some(locale("en-US")));
    }

    #[test]
    fn the_arguments_of_a_message_are_passed_on() {
        let message_source = TestMessageSource::with_message(Some("Maximum sessions reached"));
        let accessor =
            MessageSourceAccessor::new(Arc::clone(&message_source) as Arc<dyn MessageSource>);
        let sessions = 3;

        assert_eq!(
            accessor.message_or_default("exceeded", Some(&[&sessions]), "fallback"),
            "Maximum sessions reached"
        );
        assert_eq!(message_source.last_arguments(), 1);
    }

    #[test]
    fn a_lookup_without_a_message_or_a_default_answers_with_an_empty_string() {
        let accessor = MessageSourceAccessor::new(
            TestMessageSource::without_message() as Arc<dyn MessageSource>
        );

        assert_eq!(accessor.message_or_default("missing", None, "fallback"), "");
    }

    #[test]
    fn the_default_message_answers_a_lookup_that_has_no_message() {
        let accessor = MessageSourceAccessor::new(
            TestMessageSource::with_message(None) as Arc<dyn MessageSource>
        );

        assert_eq!(
            accessor.message_or_default("missing", None, "fallback"),
            "fallback"
        );
    }

    #[test]
    fn a_lookup_without_a_message_is_reported_as_an_error() {
        let message_source = TestMessageSource::without_message();
        let accessor = MessageSourceAccessor::with_default_locale(
            Arc::clone(&message_source) as Arc<dyn MessageSource>,
            locale("zh-CN"),
        );

        let error = accessor.message("missing", None).unwrap_err();

        assert_eq!(error.code, "missing");
        assert_eq!(error.locale, Some(locale("zh-CN")));
        assert_eq!(message_source.last_locale(), Some(locale("zh-CN")));
    }

    #[test]
    fn the_message_of_a_resolvable_uses_the_default_locale() {
        let message_source = TestMessageSource::with_message(Some("Resolved"));
        let accessor = MessageSourceAccessor::with_default_locale(
            Arc::clone(&message_source) as Arc<dyn MessageSource>,
            locale("fr-FR"),
        );
        let resolvable = TestResolvable::new(&["greeting"], &["Rust"], None);

        assert_eq!(
            accessor.message_from_resolvable(&resolvable).unwrap(),
            "Resolved"
        );
        assert_eq!(message_source.last_locale(), Some(locale("fr-FR")));
        assert_eq!(message_source.last_arguments(), 1);
        assert_eq!(
            accessor
                .message_from_resolvable_with_locale(&resolvable, &locale("en-GB"))
                .unwrap(),
            "Resolved"
        );
        assert_eq!(message_source.last_locale(), Some(locale("en-GB")));
    }
}
