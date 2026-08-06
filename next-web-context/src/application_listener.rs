use crate::ApplicationEvent;

pub trait ApplicationListener<E>
where
    E: ApplicationEvent,
{
    /// Handle an application event.
    fn on_application_event(&self, event: E);
}
