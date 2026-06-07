use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crate::config::security_builder::SecurityBuilder;

#[derive(Clone)]
pub struct AbstractSecurityBuilder<O>
where
    O: Send + Sync,
    Self: SecurityBuilder<O>,
{
    building: Arc<AtomicBool>,
    object: Arc<Mutex<Option<O>>>,
}

impl<O> AbstractSecurityBuilder<O>
where
    O: Send + Sync,
{
    pub fn new() -> Self {
        AbstractSecurityBuilder {
            building: Arc::new(AtomicBool::new(false)),
            object: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_object(&mut self, object: O) {
        *self.object.lock().expect("security builder lock poisoned") = Some(object);
    }
}

impl<O> SecurityBuilder<O> for AbstractSecurityBuilder<O>
where
    O: Send + Sync,
{
    fn build(&self) -> O {
        assert!(
            !self.building.swap(true, Ordering::SeqCst),
            "This object has already been built"
        );

        self.object
            .lock()
            .expect("security builder lock poisoned")
            .take()
            .expect("No object has been configured for this security builder")
    }
}
