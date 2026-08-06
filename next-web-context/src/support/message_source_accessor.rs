use std::{
    fmt::{self},
    sync::Arc,
};

use crate::MessageSource;

#[derive(Clone)]
pub struct MessageSourceAccessor {}

impl MessageSourceAccessor {
    pub fn new(message_source: Arc<dyn MessageSource>) -> Self {
        Self {}
    }

    pub fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default: &str,
    ) -> String {
        todo!()
    }
}
