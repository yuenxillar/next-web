use next_web_core::async_trait;

#[async_trait]
pub trait MessageInterceptor {
    fn order(&self) -> i32 {
        i32::MAX
    }

    async fn message_entry(&self) -> bool;

    async fn message_push(&self) -> bool;

}