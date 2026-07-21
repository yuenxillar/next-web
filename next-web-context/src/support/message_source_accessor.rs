use std::sync::Arc;

use crate::MessageSource;

#[derive(Clone)]
pub struct MessageSourceAccessor {}

impl MessageSourceAccessor {
    pub fn new(message_source: Arc<dyn MessageSource>) -> Self {
        Self {}
    }
}
