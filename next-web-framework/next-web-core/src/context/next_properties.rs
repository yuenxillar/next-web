use crate::autoconfigure::context::application_properties::AppliationProperties;
use crate::autoconfigure::context::message_source_properties::MessageSourceProperties;
use crate::autoconfigure::context::server_properties::ServerProperties;
use crate::autoconfigure::context::logger_properties::LoggerProperties;


#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct NextProperties {
    server: ServerProperties,
    appliation: Option<AppliationProperties>,
    messages: Option<MessageSourceProperties>,
    logger: Option<LoggerProperties>,
}

impl NextProperties {
    pub fn server(&self) -> &ServerProperties {
        &self.server
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
}
