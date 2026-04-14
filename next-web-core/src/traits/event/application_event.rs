use std::any::Any;

use dyn_clone::{DynClone, clone_trait_object};

pub trait ApplicationEvent
where
    Self: Send + Sync,
    Self: Any,
    Self: DynClone,
{
    fn timestamp(&self) -> u64 {
        0
    }

    fn source(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

clone_trait_object!(ApplicationEvent);

#[derive(Debug, Clone)]
pub struct EventAttributes {
    timestamp: u64,
}

impl EventAttributes {
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

impl Default for EventAttributes {
    fn default() -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl<T> ApplicationEvent for T
where
    T: AsRef<EventAttributes>,
    T: Send + Sync,
    T: Clone,
    T: Any,
{
    fn timestamp(&self) -> u64 {
        self.as_ref().timestamp()
    }
}
