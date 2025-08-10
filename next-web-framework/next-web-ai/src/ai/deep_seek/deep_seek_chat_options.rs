#[derive(Clone)]
pub struct DeepSeekChatOptions {
    model: Box<str>,
    frequency_penalty: f64,
    max_tokens: u64,
    presence_penalty: f64,
    response_format: Box<str>,
    stop: Vec<String>,
    top_p: f64,
}
