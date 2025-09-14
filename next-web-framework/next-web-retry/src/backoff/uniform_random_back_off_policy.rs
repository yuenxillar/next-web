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


impl BackOffPolicy for UniformRandomBackOffPolicy {
    fn start(
        &self,
        context: &dyn crate::retry_context::RetryContext,
    ) -> Option<&dyn super::back_off_context::BackOffContext> {
        todo!()
    }

    fn backoff(
        &self,
        context: &dyn super::back_off_context::BackOffContext,
    ) -> Result<(), crate::error::retry_error::RetryError> {
        todo!()
    }
}
