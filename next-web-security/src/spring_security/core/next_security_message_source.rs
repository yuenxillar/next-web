use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use next_web_context::{
    support::{MessageSourceAccessor, ResourceBundleMessageSource},
    Locale, MessageSource, MessageSourceResolvable, NoSuchMessageError,
};

/// The default MessageSource used by Next Security.
/// All Next Security classes requiring message localization will by default use this class. However, all such
/// classes will also implement MessageSourceAware so that the application context can inject an alternative
/// message source. Therefore this class is only used when the deployment environment has not specified an alternative message source.
#[derive(Clone, Default)]
pub struct NextSecurityMessageSource(ResourceBundleMessageSource);

impl NextSecurityMessageSource {
    pub fn get_accessor() -> MessageSourceAccessor {
        MessageSourceAccessor::new(Arc::new(Self::default()))
    }
}

impl MessageSource for NextSecurityMessageSource {
    /// Resolves the message from the resource bundles of the source, and falls
    /// back to the given default message.
    fn message_or_default(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        self.0
            .message_or_default(code, args, default_message, locale)
    }

    /// Resolves the message from the resource bundles of the source.
    ///
    /// # Errors
    ///
    /// Returns [`NoSuchMessageError`] when the bundles do not contain the code
    /// and the source does not hold a default message for it.
    fn message(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        self.0.message(code, args, locale)
    }

    /// Resolves the message the resolvable describes from the resource bundles
    /// of the source.
    ///
    /// # Errors
    ///
    /// Returns [`NoSuchMessageError`] when the bundles do not contain the codes
    /// of the resolvable and it does not carry a default message.
    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        self.0.message_from_resolvable(resolvable, locale)
    }
}

impl Debug for NextSecurityMessageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NextSecurityMessageSource")
    }
}
