use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::async_trait;

use crate::{
    listener::base_composite_listener::BaseCompositeListener,
    trigger::trigger_listener::TriggerListener,
};

/// A trigger listener which dispatches to a set of listeners.
#[derive(Clone)]
#[derive(Default)]
pub struct CompositeTriggerListener {
    base: BaseCompositeListener<Arc<dyn TriggerListener>>,
}

#[async_trait]
impl TriggerListener for CompositeTriggerListener {
    async fn triggered(&self) {
        for trigger in self.get_listeners().ordered.iter().rev() {
            trigger.triggered().await;
        }
    }
}

impl Deref for CompositeTriggerListener {
    type Target = BaseCompositeListener<Arc<dyn TriggerListener>>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CompositeTriggerListener {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

