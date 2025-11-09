pub trait SessionValidationScheduler
where
    Self: Send + Sync,
{
    fn is_enabled(&self) -> bool;

    fn enable_session_validation(&mut self);

    fn disable_session_validation(&mut self);
}
