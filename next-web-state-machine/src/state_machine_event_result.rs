//! Results reporting what happened to an event that was sent to a region.

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use next_web_core::messaging::Message;

use crate::region::Region;
use crate::support::state_machine_executor::StateMachineExecutorCallback;

/// The result type of an event that was sent into a state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultType {
    /// The event was accepted and is being processed.
    Accepted,
    /// The event was deferred by the current state and will be replayed
    /// when the machine enters a state which does not defer it.
    Deferred,
    /// The event was denied, for example because the machine is in an error
    /// state, because security rules vetoed it or because no transition
    /// matched the event.
    Denied,
}

impl ResultType {
    /// Checks whether the result type is [`ResultType::Accepted`].
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Checks whether the result type is [`ResultType::Deferred`].
    pub fn is_deferred(&self) -> bool {
        matches!(self, Self::Deferred)
    }

    /// Checks whether the result type is [`ResultType::Denied`].
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Denied)
    }
}

/// The result of sending an event into a region.
pub trait StateMachineEventResult<S, E>: Send + Sync {
    /// Gets the result type of the event.
    fn result_type(&self) -> ResultType;

    /// Gets the message which was sent.
    fn message(&self) -> Option<&Arc<dyn Message<E>>>;

    /// Gets the region which produced this result.
    fn region(&self) -> Option<&Arc<dyn Region<S, E>>>;

    /// Marks the event as fully processed.
    ///
    /// The Java implementation completes a `Mono` sink here; in Rust this is
    /// the point where an [`StateMachineExecutorCallback`] is notified.
    fn complete(&self) {}
}

/// Default implementation of [`StateMachineEventResult`].
pub struct DefaultStateMachineEventResult<S, E> {
    region: Option<Arc<dyn Region<S, E>>>,
    message: Option<Arc<dyn Message<E>>>,
    result_type: ResultType,
    callback: Option<Arc<dyn StateMachineExecutorCallback>>,
}

impl<S, E> DefaultStateMachineEventResult<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new result.
    ///
    /// # Arguments
    /// * `region` - the region which produced the result
    /// * `message` - the message which was sent
    /// * `result_type` - the result type
    pub fn new(
        region: Option<Arc<dyn Region<S, E>>>,
        message: Option<Arc<dyn Message<E>>>,
        result_type: ResultType,
    ) -> Self {
        Self {
            region,
            message,
            result_type,
            callback: None,
        }
    }

    /// Creates an accepted result.
    ///
    /// # Arguments
    /// * `region` - the region which produced the result
    /// * `message` - the message which was sent
    pub fn accepted(region: Option<Arc<dyn Region<S, E>>>, message: Arc<dyn Message<E>>) -> Self {
        Self::new(region, Some(message), ResultType::Accepted)
    }

    /// Creates a deferred result.
    ///
    /// # Arguments
    /// * `region` - the region which produced the result
    /// * `message` - the message which was deferred
    pub fn deferred(region: Option<Arc<dyn Region<S, E>>>, message: Arc<dyn Message<E>>) -> Self {
        Self::new(region, Some(message), ResultType::Deferred)
    }

    /// Creates a denied result.
    ///
    /// # Arguments
    /// * `region` - the region which produced the result
    /// * `message` - the message which was denied
    pub fn denied(region: Option<Arc<dyn Region<S, E>>>, message: Arc<dyn Message<E>>) -> Self {
        Self::new(region, Some(message), ResultType::Denied)
    }

    /// Attaches the callback which is completed when the event is processed.
    ///
    /// # Arguments
    /// * `callback` - the executor callback
    pub fn with_callback(mut self, callback: Arc<dyn StateMachineExecutorCallback>) -> Self {
        self.callback = Some(callback);
        self
    }
}

impl<S, E> StateMachineEventResult<S, E> for DefaultStateMachineEventResult<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn result_type(&self) -> ResultType {
        self.result_type
    }

    fn message(&self) -> Option<&Arc<dyn Message<E>>> {
        self.message.as_ref()
    }

    fn region(&self) -> Option<&Arc<dyn Region<S, E>>> {
        self.region.as_ref()
    }

    fn complete(&self) {
        if let Some(callback) = self.callback.as_ref() {
            callback.complete();
        }
    }
}

impl<S, E> Debug for DefaultStateMachineEventResult<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("DefaultStateMachineEventResult")
            .field("result_type", &self.result_type)
            .finish()
    }
}
