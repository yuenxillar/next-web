use crate::model::response_meta_data::ResponseMetadata;




pub struct ChatResponseMetadata {

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