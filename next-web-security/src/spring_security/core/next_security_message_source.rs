use std::{fmt, sync::Arc};

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
    fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        todo!()
    }

    fn message(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        todo!()
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        todo!()
    }
}
