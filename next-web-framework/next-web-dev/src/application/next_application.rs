use std::ops::Deref;

use hashbrown::HashMap;
use next_web_core::context::properties::ApplicationProperties;

use super::application::Application;
use next_web_core::autoconfigure::context::server_properties::ServerProperties;

#[derive(serde::Deserialize, Default)]
pub struct NextApplication<A: Application> {
    application_properties: ApplicationProperties,
    application: A,
}

impl<A: Application + Default> NextApplication<A> {
    pub fn new() -> Self {
        Self {
            application_properties: ApplicationProperties::new(),
            application: A::default(),
        }
    }

    /// Get the application register.
    pub fn application_properties(&self) -> &ApplicationProperties {
        &self.application_properties
    }

    /// Get the application name.
    pub fn application_name(&self) -> &str {
        self.application_properties()
            .next()
            .appliation()
            .map(|var| var.name())
            .unwrap_or_default()
    }

    /// Get the application context path.
    pub fn server_context_path(&mut self) -> &str {
        self.server_properties().context_path()
    }

    /// Get the application server port.
    pub fn server_port(&mut self) -> u16 {
        self.server_properties().port()
    }

    /// The function to get the application server configuration.
    pub fn server_properties(&self) -> &ServerProperties {
        self.application_properties().next().server()
    }

    /// Get the application.
    pub fn application(&mut self) -> &mut A {
        &mut self.application
    }

    /// Set the application register.
    pub fn set_application_properties(&mut self, application_properties: ApplicationProperties) {
        self.application_properties = application_properties;
    }

    /// Get the application configure mappping.
    pub fn set_configure_mappping(&mut self, mapping: HashMap<String, serde_yaml::Value>) {
        self.application_properties.set_mapping(mapping);
    }
}

impl<A: Application> Deref for NextApplication<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.application
    }
}
