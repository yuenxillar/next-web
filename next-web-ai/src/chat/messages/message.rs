use next_web_core::DynClone;

use crate::chat::messages::message_type::MessageType;

pub trait Message
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn message_type(&self) -> MessageType;

    fn text(&self) -> &[u8];
}

next_web_core::clone_trait_object!(Message);
