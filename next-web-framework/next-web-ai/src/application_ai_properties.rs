use crate::deep_seek::properties::DeepSeekProperties;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationAIProperties {
    deep_seek: Option<DeepSeekProperties>,

    
}