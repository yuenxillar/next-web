use crate::core::event::event_bus::EventBus;

pub trait EventBusAware<T: EventBus>: Send + Sync {
    fn set_event_bus(&mut self, event_bus: T);
}
