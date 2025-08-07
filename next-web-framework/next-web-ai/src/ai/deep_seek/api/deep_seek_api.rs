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
        let client = reqwest::Client::builder()
            .build()
            .unwrap();
        let api_key = api_key.into();
        Self {
            base_url: "https://api.deepseek.com/chat/completions".into(),
            api_key,
            chat_model,
            client,
        }
    }

    pub async fn send(&self, req: &ChatCompletionRequest) -> Result<ChatCompletion, BoxError> {
        let body = serde_json::to_string(req)?;
        println!("body: {:?}", body);
                let resp = self
            .client
            .post(self.base_url.as_ref())
            .header("Content-Type", "application/json")
            .bearer_auth(self.api_key.as_ref())
            .body(body)
            .send()
            .await?;
        let text = resp.text().await?;
        println!("resp: {:?}", text);
        // resp.json().await.map_err(Into::into)
        serde_json::from_str(&text).map_err(Into::into)
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


///
/// {
///  "id": "9710a6c0-1b51-427b-b95d-b6734ec46270",
///  "object": "chat.completion",
///  "created": 1754571706,
///  "model": "deepseek-chat",
///  "choices": [
///    {
///      "index": 0,
///      "message": {
///        "role": "assistant",
///        "content": "Hello! 😊 How can I assist you today?"
///      },
///      "logprobs": null,
///      "finish_reason": "stop"
///    }
///  ],
///  "usage": {
///    "prompt_tokens": 11,
///    "completion_tokens": 11,
///    "total_tokens": 22,
///    "prompt_tokens_details": {
///      "cached_tokens": 0
///    },
///    "prompt_cache_hit_tokens": 0,
///    "prompt_cache_miss_tokens": 11
///  },
///  "system_fingerprint": "fp_8802369eaa_prod0623_fp8_kvcache"
/// }
/// 
/// 
/// 
#[derive(serde::Deserialize)]
pub struct ChatCompletion {
    pub id: Box<str>,
    pub model: Box<str>,
    pub data: String,
}
