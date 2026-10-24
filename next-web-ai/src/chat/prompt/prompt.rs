use bytes::Bytes;

use crate::{
    chat::{
        messages::{message::Message, user_message::UserMessage},
        prompt::chat_options::ChatOptions,
    },
    model::{model_options::ModelOptions, model_request::ModelRequest},
};

#[derive(Clone)]
pub struct Prompt {
    messages: Vec<Box<dyn Message>>,
    chat_options: Box<dyn ChatOptions>,
}

impl Prompt {
    pub fn new(messages: Vec<Box<dyn Message>>, chat_options: Box<dyn ChatOptions>) -> Self {
        Self {
            messages,
            chat_options,
        }
    }

    pub fn from_message(message: Box<dyn Message>, chat_options: Box<dyn ChatOptions>) -> Self {
        Self::new(vec![message], chat_options)
    }

    pub fn from_contents(contents: impl Into<Bytes>, chat_options: Box<dyn ChatOptions>) -> Self {
        let message = UserMessage::new(Default::default(), contents.into(), Default::default());
        let messages: Vec<Box<dyn Message>> = vec![Box::new(message)];
        Self {
            messages,
            chat_options,
        }
    }

    pub fn chat_options(&self) -> &Box<dyn ChatOptions> {
        &self.chat_options
    }
}

impl ModelRequest<Vec<Box<dyn Message>>> for Prompt {
    fn instructions(&self) -> Vec<Box<dyn Message>> {
        todo!()
    }

    fn options(&self) -> Option<&dyn ModelOptions> {
        Some(self.chat_options.as_ref())
    }
}
