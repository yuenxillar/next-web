use next_web_core::autoconfigure::context::{application_properties::AppliationProperties, logger_properties::LoggerProperties, message_source_properties::MessageSourceProperties, server_properties::ServerProperties};

use super::{
     data_properties::DataProperties,
    security_properties::SecurityProperties,
};

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct NextProperties {
    server: ServerProperties,
    appliation: Option<AppliationProperties>,
    data: Option<DataProperties>,
    messages: Option<MessageSourceProperties>,
    logger: Option<LoggerProperties>,
    security: Option<SecurityProperties>,
}

impl NextProperties {
    pub fn server(&self) -> &ServerProperties {
        &self.server
    }

    pub fn data(&self) -> Option<&DataProperties> {
        self.data.as_ref()
    }

    pub fn messages(&self) -> Option<&MessageSourceProperties> {
        self.messages.as_ref()
    }

    pub fn appliation(&self) -> Option<&AppliationProperties> {
        self.appliation.as_ref()
    }

    pub fn logger(&self) -> Option<&LoggerProperties> {
        self.logger.as_ref()
    }

    pub fn security(&self) -> Option<&SecurityProperties> {
        self.security.as_ref()
    }
}
