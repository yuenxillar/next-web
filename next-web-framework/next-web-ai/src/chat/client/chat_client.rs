use bytes::Bytes;
use next_web_core::async_trait;
use serde::de::DeserializeOwned;

use crate::chat::{model::chat_response::ChatResponse, prompt::prompt::Prompt};

pub trait ChatClient: Send + Sync {
    fn prompt(&self, prompt: Prompt) -> impl ChatClientRequestSpec;

    fn prompt_from_content<T>(&self, message: T) -> impl ChatClientRequestSpec
    where
        T: Into<Bytes>;

    fn prompt_from_default(&self) -> impl ChatClientRequestSpec;
}

#[async_trait]
pub trait ChatClientRequestSpec: Send + Sync {
    async fn call(&self) -> impl CallResponseSpec;

    async fn stream(&self) -> impl StreamResponseSpec;

    fn user<T>(&mut self, text: T)
    where
        T: Into<Bytes>;

    fn system<T>(&mut self, text: T)
    where
        T: Into<Bytes>;
}

pub trait CallResponseSpec: Send + Sync {
    fn entity<T>(&self) -> T
    where
        T: DeserializeOwned;

    fn chat_response(&self) -> ChatResponse;

    fn content(&self) -> Bytes;

    fn response_entity<T>(&self) -> T
    where
        T: DeserializeOwned;
}

#[async_trait]
pub trait StreamResponseSpec: Send + Sync {
    async fn chat_response<S>(&self) ->  impl futures_core::Stream<Item = ChatResponse> + Send + 'static;

    async fn content<S>(&self) -> impl futures_core::Stream<Item = ChatResponse> + Send + 'static;
}
