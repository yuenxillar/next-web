pub trait AutoRegister: Sync + Send {
    fn name(&self) -> &'static str;

    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>>;
}

// 将 instance 方法移到一个单独的特质
pub trait AutoRegisterInstanceHelper: AutoRegister {
    fn instance<T, F, Fut>(&self, transform: F) -> T
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        futures::executor::block_on(transform())
    }
}

// 为所有 AutoRegister 实现 AutoRegisterInstanceHelper
impl<T: AutoRegister> AutoRegisterInstanceHelper for T {}
