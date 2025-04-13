use next_web_core::async_trait;

use rudi::Singleton;

use super::message_interceptor::MessageInterceptor;

#[Singleton(name = "defaultMQTTMessageInterceptor")]
#[derive(Clone)]
pub struct DefaultMessageInterceptor;


#[async_trait]
impl MessageInterceptor for DefaultMessageInterceptor {

    async fn message_entry(&self) -> bool {
        // 丢弃空数据

        // todo

        true
    }

    async fn message_push(&self) -> bool {
        true
    }
}