use std::sync::Arc;

use next_web_core::async_trait;

use crate::backoff::back_off_context::BackOffContext;

use super::back_off_policy::BackOffPolicy;

#[derive(Clone)]
pub struct UniformRandomBackOffPolicy {}


impl UniformRandomBackOffPolicy {
    pub fn new() -> Self {
        Self {}
    }

    pub fn set_min_back_off_period(&mut self, min_interval: u64) {


        
    }

    pub fn set_max_back_off_period(&mut self, max_interval: u64) {

        
    }
}

#[async_trait]
impl BackOffPolicy for UniformRandomBackOffPolicy {
    async fn start(
        &self,
        context: &dyn crate::retry_context::RetryContext,
    ) -> Option<Arc<dyn BackOffContext>> {
        todo!()
    }

    async fn backoff(
        &self,
        context: Option<&dyn BackOffContext>,
    ) -> Result<(), crate::error::retry_error::RetryError> {
        todo!()
    }
}
