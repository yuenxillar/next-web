use crate::{
    context::config::ConfigDataEnvironmentPostProcessor,
    env::DecryptPropertiesEnvironmentPostProcessor, ApplicationEventHandler,
    EnvironmentPostProcessor, EnvironmentPreparedPayload, Event,
};

/// The decryption must run after the config data has been loaded, so that the
/// values of the configuration files are decrypted.
const _: () = assert!(
    ConfigDataEnvironmentPostProcessor::ORDER < DecryptPropertiesEnvironmentPostProcessor::ORDER
);

#[derive(Default)]
pub struct EnvironmentPostProcessorEventHandler;

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

        // The post processors run in the order they are registered, which
        // mirrors the order of the post processors of an application: the config
        // data is loaded first, and the encrypted values it holds are decrypted
        // right after it. The orders are stated by the `Ordered` implementation
        // of each processor.
        vec![
            Box::new(post_processor),
            Box::new(DecryptPropertiesEnvironmentPostProcessor::default()),
        ]
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
