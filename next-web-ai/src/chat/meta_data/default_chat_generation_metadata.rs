use std::collections::{HashMap, HashSet};

use crate::{
    chat::meta_data::chat_generation_meta_data::ChatGenerationMetadata,
    model::result_meta_data::ResultMetadata,
};

#[derive(Clone)]
pub struct DefaultChatGenerationMetadata {
    pub(crate) metadata: Option<HashMap<String, String>>,
    pub(crate) finish_reason: Option<String>,
    pub(crate) content_filters: Option<HashSet<Box<str>>>,
}

impl DefaultChatGenerationMetadata {
    pub fn null() -> Self {
        Self {
            metadata: Default::default(),
            finish_reason: Default::default(),
            content_filters: Default::default(),
        }
    }
}

impl ResultMetadata for DefaultChatGenerationMetadata {}
impl ChatGenerationMetadata for DefaultChatGenerationMetadata {}
