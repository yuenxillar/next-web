use std::{error::Error, sync::Arc};

use crate::{
    executor::{
        context::job_context::{JobHandler, JobHandlerValue},
        model::{ExecutorActorReq, ExecutorActorResult},
    },
    web_server::state::XxlJobAppState,
};

#[derive(Clone)]
pub struct XxlClient {
    pub(crate) app_state: Arc<XxlJobAppState>,
}

impl XxlClient {
    pub fn new(app_state: Arc<XxlJobAppState>) -> Self {
        Self { app_state }
    }

    pub async fn register<S>(
        &self,
        job_name: S,
        job_handler: JobHandler,
    ) -> Result<ExecutorActorResult, Box<dyn Error>>
    where
        S: Into<String>,
    {
        let job_name = Arc::new(job_name.into());
        self.app_state
            .executor_actor
            .send(ExecutorActorReq::Register(JobHandlerValue::new(
                job_name,
                job_handler,
            )))
            .await
    }
}
