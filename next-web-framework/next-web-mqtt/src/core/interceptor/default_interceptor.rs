use next_web_core::async_trait;

use rudi_dev::Singleton;

use super::message_interceptor::MessageInterceptor;

#[Singleton(name = "defaultMQTTMessageInterceptor", binds = [Self::into_message_interceptor])]
#[derive(Clone)]
pub struct DefaultMessageInterceptor;

impl DefaultMessageInterceptor {
    fn into_message_interceptor(self) -> Box<dyn MessageInterceptor> {
        Box::new(self)
    }
}

#[async_trait]
impl MessageInterceptor for DefaultMessageInterceptor {
    async fn message_entry(&self, _topic: &str, _data: &[u8]) -> bool {
        // TODO 丢弃空数据
        true
    }
}