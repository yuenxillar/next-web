use next_web_core::async_trait;

use next_web_macros::singleton;

use super::message_interceptor::MessageInterceptor;

/// The message interceptor a client uses when the application registers none.
///
/// It lets every message through, which keeps the dispatch path open by
/// default.
#[singleton(binds = [Self::into_message_interceptor])]
#[derive(Clone)]
pub struct DefaultMessageInterceptor;

impl DefaultMessageInterceptor {
    /// Erases the interceptor into the trait object the client resolves.
    fn into_message_interceptor(self) -> Box<dyn MessageInterceptor> {
        Box::new(self)
    }
}

#[async_trait]
impl MessageInterceptor for DefaultMessageInterceptor {
    /// Lets every message through.
    async fn message_entry(&self, _topic: &str, _data: &[u8]) -> bool {
        true
    }
}
