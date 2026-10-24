use std::collections::HashMap;

use bytes::Bytes;

use crate::chat::messages::message_type::MessageType;

#[derive(Clone)]
pub struct AssistantMessage {
    pub text_content: Bytes,
    pub metadata: Option<HashMap<String, String>>,
    pub message_type: MessageType,
}
