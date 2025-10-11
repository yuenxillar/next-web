use crate::config::security_builder::SecurityBuilder;

pub trait SecurityConfigurer<O, B: SecurityBuilder<O>>
where
    Self: Send + Sync,
{
    fn init(&mut self, builer: &mut  B);

    fn configure(&mut self, builer:&mut B);
}
