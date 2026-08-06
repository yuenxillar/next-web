use std::{fmt, sync::Arc};

use next_web_context::{
    support::MessageSourceAccessor, Locale, MessageSource, MessageSourceResolvable,
    NoSuchMessageError,
};

#[derive(Clone)]
pub struct NextSecurityMessageSource;

impl NextSecurityMessageSource {
    pub fn get_accessor() -> MessageSourceAccessor {
        MessageSourceAccessor::new(Arc::new(Self))
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
