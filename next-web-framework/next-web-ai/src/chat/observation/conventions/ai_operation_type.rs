
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiOperationType {
    
    Chat,

    Embbedding,

    Framwork,

    Image,

    TextCompletion
}


impl AsRef<str> for AiOperationType {
    fn as_ref(&self) -> &str {
        match self {
            AiOperationType::Chat => "chat",
            AiOperationType::Embbedding => "embedding",
            AiOperationType::Framwork => "framework",
            AiOperationType::Image => "image",
            AiOperationType::TextCompletion => "text_completion",
        }
    }
}

impl ToString for AiOperationType {
    fn to_string(&self) -> String {
        self.as_ref().to_string()
    }
}