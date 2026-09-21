use crate::{
    context::config::ConfigDataEnvironmentPostProcessor, ApplicationEventHandler,
    EnvironmentPostProcessor, EnvironmentPreparedPayload, Event,
};

#[derive(Default)]
pub struct EnvironmentPostProcessorEventHandler {}

impl EnvironmentPostProcessorEventHandler {
    fn on_environment_prepared_event(&mut self, payload: EnvironmentPreparedPayload<'_>) {
        let mut post_processors = self.get_environment_post_processors(&payload);
        for post_processor in post_processors.iter_mut() {
            post_processor.post_process_environment(payload.environment);
        }
    }

    fn get_environment_post_processors(
        &self,
        payload: &EnvironmentPreparedPayload<'_>,
    ) -> Vec<Box<dyn EnvironmentPostProcessor>> {
        let mut post_processor = ConfigDataEnvironmentPostProcessor::default();
        if let Some(resource_loader) = payload.resource_loader.clone() {
            post_processor.set_resource_loader(resource_loader);
        }
        post_processor.set_additional_profiles(payload.additional_profiles.clone());

        vec![Box::new(post_processor)]
    }
}

impl ApplicationEventHandler for EnvironmentPostProcessorEventHandler {
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::EnvironmentPrepared(payload) => {
                self.on_environment_prepared_event(payload);
            }

            _ => {}
        }
    }
}
