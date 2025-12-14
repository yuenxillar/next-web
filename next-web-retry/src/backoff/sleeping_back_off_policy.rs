use next_web_core::async_trait;

use crate::backoff::back_off_policy::BackOffPolicy;


#[async_trait]
pub trait SleepingBackOffPolicy
where 
Self: Send + Sync,
Self: BackOffPolicy
 {
    async fn sleep(&self, sleep: u64);
}