/// AI system provided by Anthropic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiProvider {
    /// AI system provided by DeepSeek.
    DeepSeek,
    /// AI system provided by Ollama.
    Ollama,
    /// AI system provided by OpenAI.
    OpenAI,
}

impl AsRef<str> for AiProvider {
    fn as_ref(&self) -> &str {
        match self {
            AiProvider::DeepSeek => "deepseek",
            AiProvider::Ollama => "ollama",
            AiProvider::OpenAI => "openai",
        }
    }
}

impl ToString for AiProvider {
    fn to_string(&self) -> String {
        self.as_ref().to_string()
    }
}
