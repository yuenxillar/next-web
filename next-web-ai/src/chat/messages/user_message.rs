use std::collections::HashMap;

use bytes::Bytes;

use crate::chat::messages::{message::Message, message_type::MessageType};

#[derive(Clone)]
pub struct UserMessage {
    message_type: MessageType,
    text_content: Bytes,
    metadata: HashMap<String, String>,
}

impl UserMessage {
    pub fn new(
        message_type: MessageType,
        text_content: impl Into<Bytes>,
        mut metadata: HashMap<String, String>,
    ) -> Self {
        let text_content = text_content.into();
        if message_type == MessageType::User || message_type == MessageType::System {
            assert!(text_content.len() > 0);
        }

        metadata.insert("messageType".into(), message_type.as_ref().into());

        Self {
            message_type,
            text_content,
            metadata,
        }
    }

    pub fn meta_data(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl Message for UserMessage {
    fn message_type(&self) -> super::message_type::MessageType {
        self.message_type.clone()
    }

    fn text(&self) -> &str {
        std::str::from_utf8(self.text_content.as_ref()).unwrap_or("")
    }
}
