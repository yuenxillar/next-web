use next_web_core::error::BoxError;

use crate::{
    observation::{
        observation::{Context, Observation, ObservationImpl},
        observation_documentation::{BoxObservationConvention, ObservationDocumentation},
        observation_registry::ObservationRegistry,
    },
    util::key_name::KeyName,
};

use super::{
    conventions::ai_observation_attributes::AiObservationAttributes,
    default_chat_model_observation_convention::DefaultChatModelObservationConvention,
    observation_convention::ObservationConvention,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ChatModelObservationDocumentation {
    ChatModelOperation,
    LowCardinalityKeyNames(LowNames),
    HighCardinalityKeyNames(HighNames),
}

impl ObservationDocumentation for ChatModelObservationDocumentation {
    fn observation(
        &self,
        custom_convention: Option<BoxObservationConvention>,
        default_convention: Option<BoxObservationConvention>,
        context: impl Context + 'static,
        registry: Box<dyn ObservationRegistry>,
    ) -> Result<Box<dyn Observation>, BoxError> {
        if self.default_convention().is_empty() {
            return Err("No default convention provided for chat model observation".into());
        }

        let mut observation = ObservationImpl::create_not_started(
            custom_convention,
            default_convention,
            context,
            Some(registry),
        );

        if let Some(name) = self.name() {
            observation.context().set_name(name);
        }

        if let Some(contextual_name) = self.contextual_name() {
            observation.contextual_name(contextual_name);
        }

        Ok(observation)
    }

    fn default_convention(&self) -> &'static str {
        std::any::type_name::<DefaultChatModelObservationConvention>()
    }

    fn low_cardinality_key_names(&self) -> Vec<KeyName> {
        LowNames::values()
    }

    fn high_cardinality_key_names(&self) -> Vec<KeyName> {
        HighNames::values()
    }
}

impl ChatModelObservationDocumentation {
    pub fn value(&self) -> &str {
        match self {
            ChatModelObservationDocumentation::LowCardinalityKeyNames(names) => names.value(),
            ChatModelObservationDocumentation::HighCardinalityKeyNames(names) => names.value(),
            _ => "",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LowNames {
    AiOperationType,
    AiProvider,
    RequestModel,
    ResponseModel,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HighNames {
    RequestFrequencyPenalty,
    RequestMaxTokens,
    RequestPresencePenalty,
    RequestStopSequences,
    RequestTemperature,
    RequestToolNames,
    RequestTopK,
    RequestTopP,
    ResponseFinishReasons,
    ResponseId,
    UsageInputTokens,
    UsageOutputTokens,
    UsageTotalTokens,
}

impl HighNames {
    pub fn value(&self) -> &str {
        match self {
            HighNames::RequestFrequencyPenalty => {
                AiObservationAttributes::RequestFrequencyPenalty.value()
            }
            HighNames::RequestMaxTokens => AiObservationAttributes::RequestMaxTokens.value(),
            HighNames::RequestPresencePenalty => {
                AiObservationAttributes::RequestPresencePenalty.value()
            }
            HighNames::RequestStopSequences => {
                AiObservationAttributes::RequestStopSequences.value()
            }
            HighNames::RequestTemperature => AiObservationAttributes::RequestTemperature.value(),
            HighNames::RequestToolNames => AiObservationAttributes::RequestToolNames.value(),
            HighNames::RequestTopK => AiObservationAttributes::RequestTopK.value(),
            HighNames::RequestTopP => AiObservationAttributes::RequestTopP.value(),
            HighNames::ResponseFinishReasons => {
                AiObservationAttributes::ResponseFinishReasons.value()
            }
            HighNames::ResponseId => AiObservationAttributes::ResponseId.value(),
            HighNames::UsageInputTokens => AiObservationAttributes::UsageInputTokens.value(),
            HighNames::UsageOutputTokens => AiObservationAttributes::UsageOutputTokens.value(),
            HighNames::UsageTotalTokens => AiObservationAttributes::UsageTotalTokens.value(),
        }
    }

    pub fn values() -> Vec<KeyName> {
        vec![
            KeyName(HighNames::RequestFrequencyPenalty.value().into()),
            KeyName(HighNames::RequestMaxTokens.value().into()),
            KeyName(HighNames::RequestPresencePenalty.value().into()),
            KeyName(HighNames::RequestStopSequences.value().into()),
            KeyName(HighNames::RequestTemperature.value().into()),
            KeyName(HighNames::RequestToolNames.value().into()),
            KeyName(HighNames::RequestTopK.value().into()),
            KeyName(HighNames::RequestTopP.value().into()),
            KeyName(HighNames::ResponseFinishReasons.value().into()),
            KeyName(HighNames::ResponseId.value().into()),
            KeyName(HighNames::UsageInputTokens.value().into()),
            KeyName(HighNames::UsageOutputTokens.value().into()),
            KeyName(HighNames::UsageTotalTokens.value().into()),
        ]
    }
}

impl LowNames {
    pub fn value(&self) -> &str {
        match self {
            LowNames::AiOperationType => AiObservationAttributes::AiOperationType.value(),
            LowNames::AiProvider => AiObservationAttributes::AiProvider.value(),
            LowNames::RequestModel => AiObservationAttributes::RequestModel.value(),
            LowNames::ResponseModel => AiObservationAttributes::ResponseModel.value(),
        }
    }

    pub fn values() -> Vec<KeyName> {
        vec![
            KeyName(LowNames::AiOperationType.value().into()),
            KeyName(LowNames::AiProvider.value().into()),
            KeyName(LowNames::RequestModel.value().into()),
            KeyName(LowNames::ResponseModel.value().into()),
        ]
    }
}
