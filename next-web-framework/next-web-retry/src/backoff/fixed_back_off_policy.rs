use crate::backoff::back_off_policy::BackOffPolicy;

#[derive(Clone)]
pub struct FixedBackOffPolicy {}
impl FixedBackOffPolicy {
    pub fn new() -> Self {
        Self {}
    }

    pub fn set_back_off_period(&mut self, interval: u64) {}
}

impl BackOffPolicy for FixedBackOffPolicy {
    fn start(
        &self,
        context: &dyn crate::retry_context::RetryContext,
    ) -> Option<&dyn crate::backoff::back_off_context::BackOffContext> {
        todo!()
    }

    fn backoff(
        &self,
        context: &dyn crate::backoff::back_off_context::BackOffContext,
    ) -> Result<(), crate::error::retry_error::RetryError> {
        todo!()
    }
}
