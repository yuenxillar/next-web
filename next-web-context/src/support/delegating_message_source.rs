use std::{fmt::Debug, sync::Arc};

use crate::MessageSource;

#[derive(Clone, Default)]
pub struct DelegatingMessageSource {
    parent_message_source: Option<Arc<dyn MessageSource>>,
}

impl DelegatingMessageSource {
    pub fn new(parent_message_source: Option<Arc<dyn MessageSource>>) -> Self {
        Self {
            parent_message_source,
        }
    }

    pub fn set_parent_message_source(&mut self, parent_message_source: Arc<dyn MessageSource>) {
        self.parent_message_source = Some(parent_message_source);
    }
}

impl MessageSource for DelegatingMessageSource {
    fn message(
        &self,
        code: &str,
        args: Option<&[&dyn std::fmt::Display]>,
        locale: Option<&crate::Locale>,
    ) -> Result<String, crate::NoSuchMessageError> {
        todo!()
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn crate::MessageSourceResolvable,
        locale: Option<&crate::Locale>,
    ) -> Result<String, crate::NoSuchMessageError> {
        todo!()
    }

    fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn std::fmt::Display]>,
        default_message: Option<&str>,
        locale: Option<&crate::Locale>,
    ) -> Option<String> {
        todo!()
    }
}

impl Debug for DelegatingMessageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DelegatingMessageSource")
    }
}
