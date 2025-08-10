#[derive(Debug, Clone, PartialEq)]
pub enum AiObservationAttributes {
    AiOperationType,
    AiProvider,

    RequestModel,
    RequestFrequencyPenalty,
    RequestMaxTokens,
    RequestPresencePenalty,
    RequestStopSequences,
    RequestTemperature,
    RequestToolNames,
    RequestTopK,
    RequestTopP,
    RequestEmbeddingDimensions,
    RequestImageResponseFormat,
    RequestImageSize,
    RequestImageStyle,

    ResponseFinishReasons,
    ResponseId,
    ResponseModel,

    UsageInputTokens,
    UsageOutputTokens,
    UsageTotalTokens,
}

impl AiObservationAttributes {
    pub fn value(&self) -> &'static str {
        match self {
            AiObservationAttributes::AiOperationType => "gen_ai.operation.name",
            AiObservationAttributes::AiProvider => "gen_ai.system",

            AiObservationAttributes::RequestModel => "gen_ai.request.model",
            AiObservationAttributes::RequestFrequencyPenalty => "gen_ai.request.frequency_penalty",
            AiObservationAttributes::RequestMaxTokens => "gen_ai.request.max_tokens",
            AiObservationAttributes::RequestPresencePenalty => "gen_ai.request.presence_penalty",
            AiObservationAttributes::RequestStopSequences => "gen_ai.request.stop_sequences",
            AiObservationAttributes::RequestTemperature => "gen_ai.request.temperature",
            AiObservationAttributes::RequestToolNames => "spring.ai.model.request.tool.names",
            AiObservationAttributes::RequestTopK => "gen_ai.request.top_k",
            AiObservationAttributes::RequestTopP => "gen_ai.request.top_p",
            AiObservationAttributes::RequestEmbeddingDimensions => {
                "gen_ai.request.embedding.dimensions"
            }
            AiObservationAttributes::RequestImageResponseFormat => {
                "gen_ai.request.image.response_format"
            }
            AiObservationAttributes::RequestImageSize => "gen_ai.request.image.size",
            AiObservationAttributes::RequestImageStyle => "gen_ai.request.image.style",

            AiObservationAttributes::ResponseFinishReasons => "gen_ai.response.finish_reasons",
            AiObservationAttributes::ResponseId => "gen_ai.response.id",
            AiObservationAttributes::ResponseModel => "gen_ai.response.model",

            AiObservationAttributes::UsageInputTokens => "gen_ai.usage.input_tokens",
            AiObservationAttributes::UsageOutputTokens => "gen_ai.usage.output_tokens",
            AiObservationAttributes::UsageTotalTokens => "gen_ai.usage.total_tokens",
        }
    }
}
