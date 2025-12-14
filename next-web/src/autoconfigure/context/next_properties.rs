use next_web_core::autoconfigure::context::{
    application_properties::AppliationProperties, logging_properties::LoggingProperties,
    server_properties::ServerProperties,
};

use super::security_properties::SecurityProperties;

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct NextProperties {
    server: ServerProperties,
    appliation: Option<AppliationProperties>,
    logging: Option<LoggingProperties>,
    security: Option<SecurityProperties>,
}

impl NextProperties {
    pub fn server(&self) -> &ServerProperties {
        &self.server
    }

    pub fn appliation(&self) -> Option<&AppliationProperties> {
        self.appliation.as_ref()
    }

    pub fn logging(&self) -> Option<&LoggingProperties> {
        self.logging.as_ref()
    }

    pub fn security(&self) -> Option<&SecurityProperties> {
        self.security.as_ref()
    }
}
