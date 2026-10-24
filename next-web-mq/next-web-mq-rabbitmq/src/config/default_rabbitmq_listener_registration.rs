use std::ops::{Deref, DerefMut};

use crate::config::{
    base_rabbitmq_listener_registration::BaseRabbitmqListenerRegistration,
    rabbitmq_listener_registration::RabbitmqListenerRegistration,
};

/// Default listener registration wrapper.
#[derive(Clone)]
pub struct DefaultRabbitmqListenerRegistration {
    base: BaseRabbitmqListenerRegistration,
}

impl DefaultRabbitmqListenerRegistration {
    pub fn new(base: BaseRabbitmqListenerRegistration) -> Self {
        Self { base }
    }
}

impl RabbitmqListenerRegistration for DefaultRabbitmqListenerRegistration {
    fn consumer_tag(&mut self, consumer_tag: String) -> &mut dyn RabbitmqListenerRegistration {
        self.base.consumer_tag(consumer_tag)
    }

    fn exchange_type(&mut self, exchange_type: String) -> &mut dyn RabbitmqListenerRegistration {
        self.base.exchange_type(exchange_type)
    }

    fn exchange_durable(&mut self, durable: bool) -> &mut dyn RabbitmqListenerRegistration {
        self.base.exchange_durable(durable)
    }

    fn queue_durable(&mut self, durable: bool) -> &mut dyn RabbitmqListenerRegistration {
        self.base.queue_durable(durable)
    }
}

impl Deref for DefaultRabbitmqListenerRegistration {
    type Target = BaseRabbitmqListenerRegistration;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DefaultRabbitmqListenerRegistration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
