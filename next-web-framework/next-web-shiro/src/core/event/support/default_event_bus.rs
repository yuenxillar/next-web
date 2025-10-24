use crate::core::{event::event_bus::EventBus, util::destroyable::Destroyable};

#[derive(Clone)]
pub struct DefaultEventBus {}


impl EventBus for DefaultEventBus {
    fn publish(&self, event: crate::core::object::Object) {
        todo!()
    }

    fn register(&mut self, event: crate::core::object::Object) {
        todo!()
    }

    fn unregister(&mut self, event: crate::core::object::Object) {
        todo!()
    }
}

impl Destroyable for DefaultEventBus  {
    fn destroy(self) {
        todo!()
    }
}

impl Default for DefaultEventBus {
    fn default() -> Self {
        Self {  }
    }
}


