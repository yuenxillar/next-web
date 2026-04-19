use std::{error::Error, sync::Arc};

use next_web_core::{
    ApplicationContext, async_trait,
    traits::config::auto_configuration::AutoConfiguration,
};
use rudi_dev::singleton;

use crate::{
    autoconfigure::xxl_job_properties::XxlJobProperties,
    client::builder::XxlClientBuilder,
    executor::context::job_context::{AsyncJobHandler, JobHandler, SyncJobHandler},
    web_server::state::XxlJobAppState,
};

#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct XxlJobAutoConfiguration(pub XxlJobProperties);

impl XxlJobAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for XxlJobAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let xxl_client = XxlClientBuilder::from(self.0.clone()).build()?;

        for (name, handler) in ctx
            .resolve_by_type::<Arc<dyn AsyncJobHandler>>()
            .into_iter()
            .map(|h| (h.name(), JobHandler::Async(h)))
            .chain(
                ctx.resolve_by_type::<Arc<dyn SyncJobHandler>>()
                    .into_iter()
                    .map(|h| (h.name(), JobHandler::Sync(h))),
            )
        {
            xxl_client.register(name, handler).await?;
        }

        ctx.insert_singleton_with_name::<Arc<XxlJobAppState>, &'static str>(
            xxl_client.app_state.clone(),
            "xxlJobAppState",
        );
        ctx.insert_singleton_with_default_name(xxl_client);

        Ok(())
    }
}
