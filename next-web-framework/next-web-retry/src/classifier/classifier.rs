use next_web_core::async_trait;

#[async_trait]
pub trait Classifier<T, C>
where
    Self: Send + Sync
{
    async fn classify(&self, classifiable: Option<& T> ) -> C;
}
