use next_web_core::error::BoxError;

use crate::ai::deep_seek::chat_model::ChatModel;

#[derive(Clone)]
pub struct DeepSeekApi {
    pub(crate) base_url: Box<str>,
    pub(crate) api_key: Box<str>,

    pub(crate) chat_model: ChatModel,

    pub(crate) client: reqwest::Client,
}

impl DeepSeekApi {
    pub fn new(api_key: impl Into<Box<str>>, chat_model: ChatModel) -> Self {
        let client = reqwest::Client::builder().build().unwrap();
        let api_key = api_key.into();
        Self {
            base_url: "https://api.deepseek.com".into(),
            api_key,
            chat_model,
            client,
        }
    }

    pub async fn send(&self, req: &ChatCompletionRequest) -> Result<ChatCompletion, BoxError> {
        let resp = self
            .client
            .post(self.base_url.as_ref())
            .bearer_auth(self.api_key.as_ref())
            .body(serde_json::to_string(req).unwrap())
            .send()
            .await?;
        resp.json().await.map_err(Into::into)
    }
}

impl Default for DeepSeekApi {
    fn default() -> Self {
        let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap();
        Self::new(api_key, ChatModel::Chat)
    }
}

#[derive(serde::Serialize)]
pub struct ChatCompletionRequest {
    pub(crate) messages: Vec<ChatCompletionMessage>,
    pub(crate) model: Box<str>,
    pub(crate) stream: bool,
}

#[derive(serde::Serialize)]
pub struct ChatCompletionMessage {
    pub(crate) role: Box<str>,
    pub(crate) content: Box<str>,
}

#[derive(serde::Deserialize)]
pub struct ChatCompletion {
    pub data: String,
}
