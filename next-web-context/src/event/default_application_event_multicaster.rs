use std::{fmt::Debug, sync::Arc};

use crate::{
    ApplicationEvent, ApplicationListener,
    event::{ApplicationEventMulticaster, MulticastError},
};

type Listener = Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>;

#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {
    listeners: Vec<Listener>,
}

impl DefaultApplicationEventMulticaster {
    pub fn new<I>(listeners: I) -> Self
    where
        I: IntoIterator<Item = Listener>,
    {
        Self {
            listeners: listeners.into_iter().collect(),
        }
    }
}

impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    fn add_application_listener(
        &mut self,
        id: String,
        listener: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    ) {
        todo!()
    }

    fn remove_application_listener(&mut self, id: String) {
        todo!()
    }

    fn remove_all_listeners(&mut self) {
        todo!()
    }

    fn multicast_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), MulticastError> {
        todo!()
    }
}

impl Debug for DefaultApplicationEventMulticaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultApplicationEventMulticaster")
    }
}
