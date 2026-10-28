use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::{BoxedStateAction, config::action::StateMachineAction, state_context::StateContext};

/// Helpers which adapt actions to the shapes the runtime expects.
pub struct Actions;

impl Actions {
    /// Wraps an action into the boxed action type of the runtime.
    ///
    /// # Arguments
    /// * `action` - the action to wrap
    pub fn from<S, E>(action: Arc<dyn StateMachineAction<S, E>>) -> BoxedStateAction<S, E>
    where
        S: Send + 'static,
        E: Send + 'static,
    {
        Arc::new(move |ctx| {
            let action = action.clone();

            Box::pin(async move { action.execute(ctx).await })
        })
    }

    /// Wraps an action so an error action runs when it fails.
    ///
    /// # Arguments
    /// * `action` - the action to execute
    /// * `error_action` - the action executed when the first one fails
    pub fn error_calling_action<S, E>(
        action: Arc<dyn StateMachineAction<S, E>>,
        error_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> Arc<dyn StateMachineAction<S, E>>
    where
        S: 'static,
        E: 'static,
    {
        Arc::new(ErrorWrappingAction::new(action, error_action))
    }
}

/// An action which runs an error action when the wrapped action fails.
pub struct ErrorWrappingAction<S, E> {
    action: Arc<dyn StateMachineAction<S, E>>,
    error_action: Arc<dyn StateMachineAction<S, E>>,
}

impl<S, E> ErrorWrappingAction<S, E> {
    /// Creates a new wrapping action.
    ///
    /// # Arguments
    /// * `action` - the action to execute
    /// * `error_action` - the action executed when the first one fails
    pub fn new(
        action: Arc<dyn StateMachineAction<S, E>>,
        error_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> Self {
        Self {
            action,
            error_action,
        }
    }
}
#[async_trait]
impl<S, E> StateMachineAction<S, E> for ErrorWrappingAction<S, E>
where
    S: 'static,
    E: 'static,
{
    async fn execute(&self, ctx: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        match self.action.execute(ctx).await {
            Ok(()) => Ok(()),
            Err(err) => {
                let _ = self.error_action.execute(ctx).await;
                Err(err)
            }
        }
    }
}
