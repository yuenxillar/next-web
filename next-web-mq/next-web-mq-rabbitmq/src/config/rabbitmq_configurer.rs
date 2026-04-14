use next_web_core::{ApplicationContext, DynClone, clone_trait_object};

use crate::config::rabbitmq_listener_registry::RabbitmqListenerRegistry;

/// Defines callback methods to configure RabbitMQ listeners and bindings.
pub trait RabbitmqConfigurer
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn register_listeners(
        &mut self,
        ctx: &mut ApplicationContext,
        registry: &mut dyn RabbitmqListenerRegistry,
    );
}

clone_trait_object!(RabbitmqConfigurer where Self: Send + Sync);
