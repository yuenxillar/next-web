use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::state_machine::state_machine_context::StateContext;

use super::{
    state_machine_context::StateMachineKey, EventMessage, StateMachine, StateMachineAction,
    Transition,
};
use std::collections::{HashMap, HashSet};
use tokio::sync::{broadcast::Receiver, RwLock};
use tracing::error;

type Action<S, E> = Arc<RwLock<HashMap<StateMachineKey<S, E>, Box<dyn StateMachineAction<S, E>>>>>;

#[derive(Clone)]
pub struct StateMachineManager<S, E> {
    pub(crate) id: String,
    pub(crate) action: Action<S, E>,
    pub(crate) key: HashSet<Transition<S, E>>,
    // pub(crate) previous_state: Option<State<S, E>>,
}

impl<S, E> StateMachineManager<S, E>
where
    E: Send + Sync + 'static,
    E: Clone + Debug + Eq + Hash,
    S: Send + Sync + 'static,
    S: Clone + Eq + Hash,
{
    pub fn new(id: String) -> Self {
        Self {
            id,
            action: Arc::new(RwLock::new(HashMap::new())),
            key: HashSet::new(),
            // previous_state: None,
        }
    }

    pub async fn add_action<K>(&mut self, key: K, action: Box<dyn StateMachineAction<S, E>>) -> &mut Self
    where
        S: Hash + Eq + PartialEq,
        E: Hash + Eq + PartialEq,
        K: Into<StateMachineKey<S, E>>,
    {
        let key = key.into();
        let k: Transition<S, E> = Transition {
            source: key.1.clone(),
            target: key.2.clone(),
            event: key.3.clone(),
        };

        if !self.key.contains(&k) {
            self.key.insert(k);
        }

        self.action
            .write()
            .await
            .insert(key, action);
        self
    }

    pub fn start_up(
        self,
        state_machine: Arc<StateMachine<S, E>>,
        mut receiver: Receiver<EventMessage<E>>,
    ) {
        tokio::spawn(async move {
            let actions = self.action;
            let id = self.id;
            let keys = self.key;
            loop {
                match receiver.recv().await {
                    Ok(event_message) => {
                        if !state_machine.status { continue; }
                        let event = event_message.event();
                        if let Some(transition) = keys
                            .iter()
                            .filter(|item| &item.event == event)
                            .last()
                            .map(|v| v.clone())
                        {
                            let key = StateMachineKey(
                                id.to_owned(),
                                transition.source,
                                transition.target,
                                event.to_owned(),
                            );
                            if let Some(action) = actions.write().await.get_mut(&key) {
                                let context = StateContext {
                                    message: event_message,
                                    transition: action.transition(),
                                    state_machine: state_machine.clone(),
                                };

                                action.execute(context).await;

                                if let Some(lis) = &state_machine.listener {
                                    lis.state_changed(key.1, key.2, key.3).await;
                                }
                            } else {
                                if let Some(lis) = &state_machine.listener {
                                    lis.event_not_accepted(event_message).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("StateMachineManager receiver error: {}", e.to_string());
                        break;
                    }
                }
            }
        });
    }
}
