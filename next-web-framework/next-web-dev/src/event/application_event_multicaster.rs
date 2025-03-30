use super::application_event::ApplicationEvent;

pub trait ApplicationEventMulticaster: Send + Sync {
    fn add_application_listener(listener: ApplicationListener<ApplicationEvent>);
    fn remove_application_listener(listener: ApplicationListener<ApplicationEvent>);
    fn multicast_event(event: ApplicationEvent);
}