use crate::config::security_configurer::SecurityConfigurer;

pub trait WebSecurityConfigurer<T>
where
    Self: Send + Sync,
    T: SecurityConfigurer<T>,
{
}
