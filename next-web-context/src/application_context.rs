use crate::{ApplicationEventPublisher, MessageSource};

pub trait ApplicationContext
where
    Self: Send + Sync,
    Self: ApplicationEventPublisher,
    Self: MessageSource,
{
    /// Return the unique id of this application context.
    fn id(&self) -> &str;

    /// Return a name for the deployed application that this context belongs to.
    fn application_name(&self) -> &str;

    /// Return the timestamp when this context was first loaded.
    fn startup_date(&self) -> i64;
}
