use std::sync::Arc;

use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
    error::BoxError,
};
use rudi_dev::singleton;

use crate::{
    autoconfigure::xxl_job_properties::XxlJobProperties,
    client::builder::XxlClientBuilder,
    executor::context::job_context::{AsyncJobHandler, JobHandler},
    web_server::state::XxlJobAppState,
};

#[singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct XxlJobAutoConfiguration(pub XxlJobProperties);

impl XxlJobAutoConfiguration {
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for XxlJobAutoConfiguration {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), BoxError> {
        let xxl_client = XxlClientBuilder::from(self.0.clone()).build()?;
        for handler in ctx.resolve_by_type::<Arc<dyn AsyncJobHandler>>() {
            let name = handler.name();
            xxl_client
                .register(name, JobHandler::Async(handler))
                .await
                .unwrap();
        }

        ctx.insert_singleton_with_name::<Arc<XxlJobAppState>, String>(
            xxl_client.app_state.clone(),
            "xxlJobAppState".into(),
        );
        ctx.insert_singleton_with_default_name(xxl_client);

        Ok(())
    }
}
