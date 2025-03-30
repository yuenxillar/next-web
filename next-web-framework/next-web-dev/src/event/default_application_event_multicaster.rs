use super::application_event_multicaster::ApplicationEventMulticaster;



#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {}


impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    fn add_application_listener(listener: ApplicationListener<super::application_event::ApplicationEvent>) {
        todo!()
    }

    fn remove_application_listener(listener: ApplicationListener<super::application_event::ApplicationEvent>) {
        todo!()
    }

    fn multicast_event(event: super::application_event::ApplicationEvent) {
        todo!()
    }
}