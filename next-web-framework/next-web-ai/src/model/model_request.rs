use crate::model::model_options::ModelOptions;



pub trait ModelRequest<T>: Send + Sync {
    fn instructions(&self) -> T;

    fn options(&self) -> Box<dyn ModelOptions>;
}