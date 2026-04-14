use next_web_core::{ApplicationContext, DynClone, clone_trait_object};

use crate::config::rocketmq_listener_registry::RocketmqListenerRegistry;

/// Callback contract used to register RocketMQ listeners declaratively.
///
/// Application modules can implement this trait and use the provided registry
/// to declare topic subscriptions in the same way Spring starter modules expose
/// `*Configurer` extension points.
pub trait RocketmqConfigurer
where
    Self: DynClone,
    Self: Send + Sync,
{
    /// Registers the listeners owned by the current module.
    fn register_listeners(
        &mut self,
        ctx: &mut ApplicationContext,
        registry: &mut dyn RocketmqListenerRegistry,
    );
}

clone_trait_object!(RocketmqConfigurer where Self: Send + Sync);
