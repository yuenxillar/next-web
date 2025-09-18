pub trait Classifier<T, C>
where
    Self: Send + Sync
{
    fn classify(&self, classifiable: Option<& T> ) -> C;
}
