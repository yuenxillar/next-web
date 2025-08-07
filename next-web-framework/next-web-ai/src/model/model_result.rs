use crate::model::result_meta_data::ResultMetadata;

pub trait ModelResult<T>: Send {
    
    fn output(&self) -> &T;

    fn meta_data(&self) -> Box<dyn ResultMetadata>;
}