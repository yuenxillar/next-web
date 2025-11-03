use std::sync::{atomic::AtomicBool, Arc};

use crate::config::security_builder::SecurityBuilder;

#[derive(Clone)]
pub struct AbstractSecurityBuilder<O>
where
    O: Send + Sync,
    Self: SecurityBuilder<O>,
{
    building: Arc<AtomicBool>,
    object: Option<O>,
}

impl<O> AbstractSecurityBuilder<O>
where
    O: Send + Sync,
{
    pub fn new() -> Self {
        AbstractSecurityBuilder {
            building: Arc::new(AtomicBool::new(false)),
            object: None,
        }
    }
}

impl<O> SecurityBuilder<O> for AbstractSecurityBuilder<O>
where
    O: Send + Sync,
{
    fn build(&self) -> O {
        todo!()
    }
}
