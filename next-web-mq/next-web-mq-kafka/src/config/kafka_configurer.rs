use next_web_core::{ApplicationContext, DynClone, clone_trait_object};

use crate::config::kafka_listener_registry::KafkaListenerRegistry;

/// Callback contract used to register Kafka listeners declaratively.
pub trait KafkaConfigurer
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn register_listeners(
        &mut self,
        ctx: &mut ApplicationContext,
        registry: &mut dyn KafkaListenerRegistry,
    );
}

clone_trait_object!(KafkaConfigurer where Self: Send + Sync);
