use bytes::Bytes;
use next_web_core::async_trait;
use serde::de::DeserializeOwned;

use crate::{
    chat::{
        client::{
            advisor::api::advisor::Advisor,
            chat_client::{
                CallResponseSpec, ChatClient, ChatClientRequestSpec, StreamResponseSpec,
            },
            response_entity::ResponseEntity,
        },
        messages::message::Message,
        model::{chat_model::ChatModel, chat_response::ChatResponse},
        prompt::{chat_options::ChatOptions, prompt::Prompt},
    },
    convert::converter::{Converter, StructuredOutputConverter},
    model::model_request::ModelRequest,
};

pub struct DefaultChatClient {
    pub(crate) default_chat_client_request: DefaultChatClientRequestSpec,
}

impl ChatClient for DefaultChatClient {
    fn prompt(&self, prompt: Prompt) -> impl super::chat_client::ChatClientRequestSpec {
        let mut spec = self.default_chat_client_request.clone();

        // Options
        spec.chat_options = prompt.chat_options().to_owned();

        // Messages
        if !prompt.instructions().is_empty() {
            spec.messages = prompt.instructions();
        }

        spec
    }

    fn prompt_from_content<T>(&self, message: T) -> impl super::chat_client::ChatClientRequestSpec
    where
        T: Into<bytes::Bytes>,
    {
        let message = message.into();
        assert!(message.len() > 0);
        self.prompt(Prompt::from_contents(
            message,
            self.default_chat_client_request.chat_options.clone(),
        ))
    }

    fn prompt_from_default(&self) -> impl super::chat_client::ChatClientRequestSpec {
        self.default_chat_client_request.clone()
    }
}

#[derive(Clone)]
pub struct DefaultChatClientRequestSpec {
    pub(crate) chat_model: Box<dyn ChatModel>,
    pub(crate) messages: Vec<Box<dyn Message>>,
    pub(crate) advisors: Vec<Box<dyn Advisor>>,

    pub(crate) user_text: Bytes,
    pub(crate) system_text: Bytes,
    pub(crate) chat_options: Box<dyn ChatOptions>,
}

#[async_trait]
impl ChatClientRequestSpec for DefaultChatClientRequestSpec {
    async fn call(&self) -> impl CallResponseSpec {
        DefaultCallResponseSpec {
            request: self.clone(),
        }
    }

    async fn stream(&self) -> impl StreamResponseSpec {
        DefaultStreamResponseSpec {
            request: self.clone(),
        }
    }

    fn user<T>(&mut self, text: T)
    where
        T: Into<Bytes>,
    {
        let text = text.into();
        assert!(text.len() > 0);
        self.user_text = text;
    }

    fn system<T>(&mut self, text: T)
    where
        T: Into<Bytes>,
    {
        let text = text.into();
        assert!(text.len() > 0);
        self.system_text = text;
    }
}

pub struct DefaultCallResponseSpec {
    pub(crate) request: DefaultChatClientRequestSpec,
}

impl DefaultCallResponseSpec {
    fn do_response_entity<T, C>(&self, output_converter: C) -> ResponseEntity<ChatResponse, T>
    where
        C: StructuredOutputConverter<T>,
    {
     
        let resp_content = Bytes::from_static(b"bytes");
        let entity = output_converter.convert(resp_content);
        ResponseEntity {
            response: todo!(),
            entity,
        }
    }
}

impl CallResponseSpec for DefaultCallResponseSpec {
    fn entity<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        todo!()
    }

    fn chat_response(&self) -> crate::chat::model::chat_response::ChatResponse {
        todo!()
    }

    fn content(&self) -> Bytes {
        todo!()
    }

    fn response_entity<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        todo!()
    }
}

pub struct DefaultStreamResponseSpec {
    request: DefaultChatClientRequestSpec,
}

#[async_trait]
impl StreamResponseSpec for DefaultStreamResponseSpec {
    async fn chat_response<S>(&self) -> S
    where
        S: futures_core::Stream<Item = ChatResponse>,
        S: Send + 'static,
    {
    }

    async fn content<S>(&self) -> S
    where
        S: futures_core::Stream<Item = Bytes>,
        S: Send + 'static,
    {
    }
}
