use crate::config::security_builder::SecurityBuilder;

pub trait SecurityConfigurer<O, B: SecurityBuilder<O>>
where
    Self: Send + Sync,
{
    fn init(&self, builer: B);

    fn configure(&self, builer: B);
}
