use std::collections::HashMap;

use crate::chat::messages::message_type::MessageType;

#[derive(Clone)]
pub struct AssistantMessage {
    pub text_content: String,
    pub metadata: Option<HashMap<String, String>>,
    pub message_type: MessageType,
}
