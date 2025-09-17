use crate::backoff::back_off_policy::BackOffPolicy;


pub trait SleepingBackOffPolicy
where 
Self: Send + Sync,
Self: BackOffPolicy
 {
    fn sleep(&self, sleep: u64);
}