use crate::configurer::idempotency_configurer::IdempotencyConfigurer;

pub struct HttpMethodHandlerConfigurer {
    pub(crate) idempotency_configurer: Option<IdempotencyConfigurer>,
}

impl HttpMethodHandlerConfigurer {
    pub fn with_idempotency_configurer(mut self, idempotency_configurer: IdempotencyConfigurer) -> Self {
        self.idempotency_configurer = Some(idempotency_configurer);
        self
    }


}

impl Default for HttpMethodHandlerConfigurer {
    fn default() -> Self {
        Self {
            idempotency_configurer: Default::default(),
        }
    }
}


#[derive(Default)]
pub struct RouterContext {
    is_idempotency: bool,
}