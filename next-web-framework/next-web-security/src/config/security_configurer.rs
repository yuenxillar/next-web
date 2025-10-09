pub trait SecurityConfigurer<B>
where
    Self: Send + Sync,
{
    fn init(&self, builer: B);

    fn configure(&self, builer: B);
}
