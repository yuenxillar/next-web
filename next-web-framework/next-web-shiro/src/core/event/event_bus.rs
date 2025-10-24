use crate::core::object::Object;

pub trait EventBus: Send + Sync 
{
    fn publish(&self, event: Object);

    fn register(&mut self, event: Object);

    fn unregister(&mut self, event: Object);
}