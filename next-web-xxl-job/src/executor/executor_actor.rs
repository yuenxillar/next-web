use std::collections::HashMap;
use std::sync::Arc;

use axum::BoxError;
use tokio::sync::Mutex;

use crate::executor::admin_server::{ServerAccessActor, callback};
use crate::executor::context::job_context::{
    JobContext, JobHandler, JobHandlerRunParam, JobHandlerValue,
};
use crate::executor::model::{ExecutorActorReq, ExecutorActorResult};
use crate::web_server::config::client_config::ClientConfig;
use crate::web_server::models::api_result::FAIL_CODE;
use crate::web_server::models::enum_type::ExecutorBlockStrategy;

#[derive(Clone, Default)]
pub struct ExecutorActor {
    #[allow(unused)]
    client_config: Arc<ClientConfig>,
    job_handler_map: Arc<Mutex<HashMap<Arc<String>, JobHandlerValue>>>,
    job_id_map: Arc<Mutex<HashMap<u64, Arc<String>>>>,
    server_access_actor: Option<ServerAccessActor>,
}

impl ExecutorActor {
    pub fn new(client_config: Arc<ClientConfig>) -> Self {
        Self {
            client_config,
            job_handler_map: Arc::new(Mutex::new(HashMap::new())),
            job_id_map: Arc::new(Mutex::new(HashMap::new())),
            server_access_actor: None,
        }
    }

    pub async fn send(&self, msg: ExecutorActorReq) -> Result<ExecutorActorResult, BoxError> {
        match msg {
            ExecutorActorReq::Register(job_handler_value) => {
                self.register_job_handler(job_handler_value).await;
                Ok(ExecutorActorResult::Ok)
            }
            ExecutorActorReq::RunJob {
                job_name,
                job_context,
            } => self.run_job(Arc::new(job_name), job_context).await,
            ExecutorActorReq::IdleBeat { job_id } => self.check_idle_beat(job_id).await,
        }
    }

    async fn register_job_handler(&self, job_handler: JobHandlerValue) {
        self.job_handler_map
            .lock()
            .await
            .insert(job_handler.name.clone(), job_handler);
    }

    async fn run_job(
        &self,
        job_name: Arc<String>,
        job_context: JobContext,
    ) -> Result<ExecutorActorResult, BoxError> {
        enum RunDecision {
            Start(JobHandlerRunParam),
            Queued(Option<JobContext>),
            Discard(JobContext),
            MissingHandler,
        }

        self.job_id_map
            .lock()
            .await
            .entry(job_context.job_id)
            .or_insert_with(|| job_name.clone());

        let mut job_context = Some(job_context);
        let decision = {
            let mut handler_map = self.job_handler_map.lock().await;

            if let Some(handler_value) = handler_map.get_mut(&job_name) {
                if handler_value.is_running {
                    match job_context
                        .as_ref()
                        .map(|ctx| &ctx.block_strategy)
                        .unwrap_or(&ExecutorBlockStrategy::Other)
                    {
                        ExecutorBlockStrategy::SerialExecution => RunDecision::Queued(
                            handler_value.push_block_job(job_context.take().unwrap()),
                        ),
                        ExecutorBlockStrategy::DiscardLater => {
                            RunDecision::Discard(job_context.take().unwrap())
                        }
                        ExecutorBlockStrategy::CoverEarly | ExecutorBlockStrategy::Other => {
                            let current_log_id = job_context
                                .as_ref()
                                .map(|ctx| ctx.log_id)
                                .unwrap_or_default();
                            handler_value.is_running = true;
                            handler_value.last_run_id = current_log_id;
                            RunDecision::Start(handler_value.build_run_param())
                        }
                    }
                } else {
                    let current_log_id = job_context
                        .as_ref()
                        .map(|ctx| ctx.log_id)
                        .unwrap_or_default();
                    handler_value.is_running = true;
                    handler_value.last_run_id = current_log_id;
                    RunDecision::Start(handler_value.build_run_param())
                }
            } else {
                RunDecision::MissingHandler
            }
        };

        match decision {
            RunDecision::Start(run_param) => {
                let actor = self.clone();
                tokio::spawn(Self::do_run_job(
                    actor,
                    job_context.take().unwrap(),
                    run_param,
                ));
                Ok(ExecutorActorResult::Ok)
            }
            RunDecision::Queued(Some(old_job)) => {
                old_job.callback_failed().await;
                Ok(ExecutorActorResult::Ok)
            }
            RunDecision::Queued(None) => Ok(ExecutorActorResult::Ok),
            RunDecision::Discard(job_context) => {
                job_context
                    .callback_failed_with_info(
                        format!(
                            "Discard the job; job_id:{}, log_id:{}",
                            job_context.job_id, job_context.log_id
                        ),
                        FAIL_CODE,
                    )
                    .await;
                Ok(ExecutorActorResult::Discard)
            }
            RunDecision::MissingHandler => Err(format!(
                "No handler registered for job: {}",
                job_name.as_str()
            )
            .as_str()
            .into()),
        }
    }

    async fn do_run_job(
        actor: Self,
        mut job_context: JobContext,
        mut job_handler_param: JobHandlerRunParam,
    ) {
        loop {
            let job_name = job_handler_param.name.clone();
            let log_id = job_context.log_id;

            let result = match job_handler_param.handler.clone() {
                JobHandler::Async(handler) => handler.process(job_context).await,
                JobHandler::Sync(handler) => {
                    match tokio::task::spawn_blocking(move || handler.process(job_context)).await {
                        Ok(result) => result,
                        Err(err) => Err(err.into()),
                    }
                }
            };

            match result {
                Ok(job) => {
                    job.callback_success().await;
                }
                Err(err) => {
                    if let Some(addr) = actor.server_access_actor.as_ref() {
                        callback(addr, log_id, FAIL_CODE, Some(err.to_string())).await;
                    }
                }
            };

            let next_job = {
                let mut handler_map = actor.job_handler_map.lock().await;
                if let Some(value) = handler_map.get_mut(&job_name) {
                    if value.last_run_id == log_id {
                        value.is_running = false;
                        value.last_run_id = 0;
                    }

                    if let Some(next_job) = value.pop_block_job() {
                        value.is_running = true;
                        value.last_run_id = next_job.log_id;
                        Some((next_job, value.build_run_param()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((next_context, next_param)) = next_job {
                job_context = next_context;
                job_handler_param = next_param;
            } else {
                break;
            }
        }
    }

    async fn check_idle_beat(&self, job_id: u64) -> Result<ExecutorActorResult, BoxError> {
        let job_name = { self.job_id_map.lock().await.get(&job_id).cloned() };

        if let Some(name) = job_name {
            if let Some(handler) = self.job_handler_map.lock().await.get(&name) {
                if handler.is_running || !handler.block_jobs.is_empty() {
                    return Ok(ExecutorActorResult::JobRunning);
                }
            }
        }
        Ok(ExecutorActorResult::Ok)
    }
}
