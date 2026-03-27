pub trait StateMachineEventResult<S, E>
where
    Self: Send + Sync,
{
}

pub struct DefaultStateMachineEventResult<S, E> {
    state: Option<S>,
    event: Option<E>,
    accepted: bool,
}

impl<S, E> DefaultStateMachineEventResult<S, E> {
    pub fn new(state: Option<S>, event: Option<E>, accepted: bool) -> Self {
        Self {
            state,
            event,
            accepted,
        }
    }

    pub fn state(&self) -> Option<&S> {
        self.state.as_ref()
    }

    pub fn event(&self) -> Option<&E> {
        self.event.as_ref()
    }

    pub fn is_accepted(&self) -> bool {
        self.accepted
    }
}

impl<S, E> StateMachineEventResult<S, E> for DefaultStateMachineEventResult<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
}
