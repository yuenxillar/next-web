use std::str::FromStr;


#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum MessageType {
    #[default]
    User,
    Assistant,
    System,
    Tool,
}


impl FromStr for MessageType {

    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(MessageType::User),
            "assistant" => Ok(MessageType::Assistant),
            "system" => Ok(MessageType::System),
            "tool" => Ok(MessageType::Tool),
            _ => Err("Invalid message type"),
        }
    }
}

impl AsRef<str> for MessageType {
    fn as_ref(&self) -> &str {
        match self {
            MessageType::User => "user",
            MessageType::Assistant => "assistant",
            MessageType::System => "system",
            MessageType::Tool => "tool",
        }
    }
}