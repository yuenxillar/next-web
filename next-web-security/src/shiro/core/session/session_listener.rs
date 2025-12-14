use crate::core::session::Session;

pub trait SessionListener
where
    Self: Send + Sync,
{
    fn on_start(&self, session: &dyn Session);

    fn on_stop(&self, session: &dyn Session);

    fn on_expiration(&self, session: &dyn Session);
}
