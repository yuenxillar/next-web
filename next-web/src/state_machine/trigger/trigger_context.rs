pub trait TriggerContext<S, E>
where
    Self: Send + Sync,
{
    fn get_event(&self) -> &E;
}
