use crate::{chat::meta_data::usage::Usage, model::response_meta_data::ResponseMetadata};

#[derive(Clone)]
pub struct ChatResponseMetadata {
    pub id: Box<str>,
    pub model: String,
    pub usage: Option<Box<dyn Usage>>,
}

impl ChatResponseMetadata {
    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn model(&self) -> &str {
        self.model.as_ref()
    }

    pub fn usage(&self) -> Option<&dyn Usage> {
        self.usage.as_deref()
    }
}

impl ResponseMetadata for ChatResponseMetadata {
    fn get<T>(&self, key: impl AsRef<str>) -> T {
        todo!()
    }

    fn get_or_default<T>(&self, key: impl AsRef<str>, default: T) -> T {
        todo!()
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

impl Default for ChatResponseMetadata {
    fn default() -> Self {
        Self {
            id: Default::default(),
            model: Default::default(),
            usage: Default::default(),
        }
    }
}
