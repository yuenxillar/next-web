use std::sync::Arc;

use crate::{authorization::authentication_manager::AuthenticationManager, config::security_builder::SecurityBuilder};


pub trait ProviderManagerBuilder<B> 
where 
    B: ProviderManagerBuilder<B>,
    Self: SecurityBuilder<Arc<dyn AuthenticationManager>>
{
    
}