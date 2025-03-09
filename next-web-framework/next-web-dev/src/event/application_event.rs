use std::{any::Any, sync::Arc};

pub trait ApplicationEvent: Any + Send + Sync + 'static {
    fn get_timestamp(&self) -> u128;
}

impl ApplicationEvent for Arc<dyn ApplicationEvent> {
    fn get_timestamp(&self) -> u128 {
        0
    }
}
