use std::sync::Arc;

use crate::{
    authorization::AuthenticationManager,
    config::security_builder::SecurityBuilder,
};

pub trait ProviderManagerBuilder<B>
where
    B: ProviderManagerBuilder<B>,
    Self: SecurityBuilder<Arc<dyn AuthenticationManager>>,
{
}
