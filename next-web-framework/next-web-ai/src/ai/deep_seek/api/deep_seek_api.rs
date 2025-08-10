use futures_core::stream::BoxStream;
use futures_util::StreamExt;
use next_web_core::error::BoxError;

use crate::ai::deep_seek::chat_model::ChatModel;

const ERROR: [u8; 8] = [123, 34, 101, 114, 114, 111, 114, 34];
const DATA: [u8; 6] = [100, 97, 116, 97, 58, 32];

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
            base_url: "https://api.deepseek.com/chat/completions".into(),
            api_key,
            chat_model,
            client,
        }
    }

    pub(crate) async fn send(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatApiRespnose, BoxError> {
        let resp = self
            .client
            .post(self.base_url.as_ref())
            .header("Content-Type", "application/json")
            .bearer_auth(self.api_key.as_ref())
            .body(serde_json::to_string(req)?)
            .send()
            .await?;
        if !req.stream {
            return Ok(ChatApiRespnose::Entity(resp.json().await?));
        }

        // SSE stream
        let stream = resp.bytes_stream().then(|data| async move {
            data.map(|data| {
                // Check for error message
                if data.starts_with(&ERROR) {
                    return Err(String::from_utf8(data.to_vec())
                        .unwrap_or("Unknown error".into())
                        .into());
                }

                data.split(|&s| s == b'\n')
                    .filter(|line| line.starts_with(&DATA))
                    .map(|line| {
                        serde_json::from_slice::<ChatCompletion>(&line[6..]).map_err(Into::into)
                    })
                    .collect::<Result<Vec<ChatCompletion>, BoxError>>()
                    .map_err(Into::into)
            })
            .unwrap_or_else(|err| Err(err.into()))
        });

        Ok(ChatApiRespnose::Stream(Box::pin(stream)))
    }
}

pub(crate) enum ChatApiRespnose {
    Entity(ChatCompletion),
    Stream(BoxStream<'static, Result<Vec<ChatCompletion>, BoxError>>),
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, serde::Deserialize)]
pub struct ChatCompletion {
    pub id: Box<str>,
    pub object: Box<str>,
    pub created: u64,
    pub model: Box<str>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub system_fingerprint: Box<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: PromptTokensDetails,
    pub prompt_cache_hit_tokens: u32,
    pub prompt_cache_miss_tokens: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct PromptTokensDetails {
    pub cached_tokens: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct Choice {
    pub index: u32,
    pub delta: Option<DeltaContent>,
    pub message: Option<ChatCompletionMessage>,
    pub logprobs: Option<Box<str>>,
    pub finish_reason: Option<Box<str>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct DeltaContent {
    pub content: Box<str>,
}
