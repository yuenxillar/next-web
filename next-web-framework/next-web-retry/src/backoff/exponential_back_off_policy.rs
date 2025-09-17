use crate::backoff::{
    back_off_policy::BackOffPolicy, sleeping_back_off_policy::SleepingBackOffPolicy,
};

#[derive(Clone)]
pub struct ExponentialBackOffPolicy {
    initial_interval: u64,
    max_interval: u64,
    multiplier: f32,
    with_random: bool,
}

impl ExponentialBackOffPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_random() -> Self {
        Self {
            with_random: true,
            ..Default::default()
        }
    }

    pub fn set_initial_interval(&mut self, initial_interval: u64) {
        self.initial_interval = initial_interval;
    }

    pub fn set_max_interval(&mut self, max_interval: u64) {
        self.max_interval = max_interval;
    }

    pub fn set_multiplier(&mut self, multiplier: f32) {
        self.multiplier = multiplier;
    }
}

impl BackOffPolicy for ExponentialBackOffPolicy {
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

impl SleepingBackOffPolicy for ExponentialBackOffPolicy {
    fn sleep(&self, sleep: u64) {
        todo!()
    }
}

impl Default for ExponentialBackOffPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Default::default(),
            max_interval: Default::default(),
            multiplier: Default::default(),
            with_random: Default::default(),
        }
    }
}
