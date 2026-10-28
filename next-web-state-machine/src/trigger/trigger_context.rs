/// The context a trigger is evaluated with.
pub trait TriggerContext<S, E>
where
    Self: Send + Sync,
{
    /// Gets the event which is being processed.
    fn get_event(&self) -> &E;
}
