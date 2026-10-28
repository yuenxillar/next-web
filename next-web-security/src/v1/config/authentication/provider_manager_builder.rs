use std::sync::Arc;

use crate::{
    authentication::AuthenticationProvider, authorization::AuthenticationManager,
    config::security_builder::SecurityBuilder,
};

pub trait ProviderManagerBuilder<B>
where
    B: ProviderManagerBuilder<B>,
    Self: SecurityBuilder<Arc<dyn AuthenticationManager>>,
{
    /// Add authentication based upon the custom AuthenticationProvider that is passed in. Since the AuthenticationProvider implementation is unknown,
    ///  all customizations must be done externally and the ProviderManagerBuilder is returned immediately.
    fn authentication_provider(
        &mut self,
        authentication_provider: Arc<dyn AuthenticationProvider>,
    ) -> &mut B;
}
