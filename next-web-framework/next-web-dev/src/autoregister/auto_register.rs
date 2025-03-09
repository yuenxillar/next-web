pub trait AutoRegister: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>>;
}
