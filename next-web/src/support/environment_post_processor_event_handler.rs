use crate::ApplicationEventHandler;

pub struct EnvironmentPostProcessorEventHandler {}

impl ApplicationEventHandler for EnvironmentPostProcessorEventHandler {
    fn handle_event(&mut self, event: crate::Event) {}
}
