use next_web_core::DynClone;

use crate::chat::messages::message_type;

pub trait Message: DynClone + Send + Sync {
    fn message_type(&self) -> message_type::MessageType;

    fn text(&self) -> &str;
}

next_web_core::clone_trait_object!(Message);
