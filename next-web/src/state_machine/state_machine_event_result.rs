pub trait StateMachineEventResult<S, E>
where
    Self: Send + Sync,
{
}

pub struct DefaultStateMachineEventResult<S, E> {
    var1: S,
    var2: E,
}

impl<S, E> StateMachineEventResult<S, E> for DefaultStateMachineEventResult<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
}
