use std::ops::Deref;

use next_web_core::{constants::application_constants::SECURE_PROPERTIES_MARK, context::{
    application_args::ApplicationArgs, application_resources::ApplicationResources,
    properties::ApplicationProperties,
}};

use super::application::Application;
use next_web_core::autoconfigure::context::server_properties::ServerProperties;

#[derive(Default)]
pub struct NextApplication<A: Application> {
    pub(crate) application_properties: ApplicationProperties,
    pub(crate) application_args: ApplicationArgs,
    pub(crate) application_resources: ApplicationResources,
    pub(crate) application: A,
}

impl<A: Application + Default> NextApplication<A> {
    pub fn new() -> Self {
        let application_args = ApplicationArgs::default();
        let application_resources = ApplicationResources::default();
        let application_properties =
            ApplicationProperties::from((&application_args, &application_resources));

        Self {
            application_properties,
            application_args,
            application_resources,
            application: A::default(),
        }
    }

    /// Get the application register.
    pub fn application_properties(&self) -> &ApplicationProperties {
        &self.application_properties
    }

    /// Get the application args
    pub fn application_args(&self) -> &ApplicationArgs {
        &self.application_args
    }

    /// Get the application resources.
    pub fn application_resources(&self) -> &ApplicationResources {
        &self.application_resources
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
    pub fn server_context_path(&mut self) -> Option<&str> {
        self.server_properties().context_path()
    }

    /// Get the application server port.
    pub fn server_port(&mut self) -> Option<u16> {
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
    pub fn set_configure_mappping(&mut self, mapping: serde_yaml::Value) {
        self.application_properties.set_mapping(mapping);
    }

    /// Decrypt the properties.
    pub(crate) fn decrypt_properties(&mut self) {
        // 不解密关于 Server 相关的配置

        if let Some(mapping) = self.application_properties.mapping_mut() {
                // println!("mapping: {:#?}", mapping);
                
                match mapping.as_mapping_mut() {
                    Some(mapping) => {
                        let var = mapping.iter_mut().filter(|(_key, value)| 
                            {
                                match value {
                                    serde_yaml::Value::String(s) => {
                                        if s.starts_with(SECURE_PROPERTIES_MARK) && &s[3..4] == ":" {
                                            
                                        }

                                        true
                                    },
                                    serde_yaml::Value::Mapping(mapping) => todo!(),
                                    _ => false
                                }                     
                            }
                        );
                    },
                    None => {}
                }
        }
    }
}

impl<A: Application> Deref for NextApplication<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.application
    }
}
