use crate::{chat::meta_data::usage::Usage, model::response_meta_data::ResponseMetadata};



#[derive(Clone)]
pub struct ChatResponseMetadata {
    pub id: Box<str>,
    pub model: Box<str>,
    pub usage: Box<dyn Usage>,
}


impl ChatResponseMetadata {
    
    pub fn id(&self) -> &str { self.id.as_ref() } 

    pub fn model(&self) -> &str { self.model.as_ref() } 

    pub fn usage(&self) -> &dyn Usage { self.usage.as_ref() } 
}
impl ResponseMetadata for  ChatResponseMetadata {
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