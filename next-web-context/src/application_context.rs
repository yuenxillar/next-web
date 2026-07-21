use crate::ApplicationEventPublisher;

pub trait ApplicationContext
where
    Self: Send + Sync,
    Self: ApplicationEventPublisher,
{
    fn id(&self) -> &str;

    fn application_name(&self) -> &str;

    fn startup_date(&self) -> i64;
}
