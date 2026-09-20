use crate::ApplicationEventHandler;

pub struct LoggingEventHandler {}

impl ApplicationEventHandler for LoggingEventHandler {
    fn handle_event(&mut self, event: crate::Event) {}
}
